use anyhow::{Context, Result};
use ignore::WalkBuilder;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tree_sitter::{Language, Node, Parser, Query};

// Helper struct to reduce function argument count
struct AstNodeParams<'a> {
    language: &'a str,
    kind: &'a str,
    line_num: usize,
    line: &'a str,
    capture: &'a str,
    match_start: usize,
    match_end: usize,
}

/// A representation of code structure that will be sent to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAST {
    pub path: String,
    pub language: String,
    pub kind: String,
    pub name: Option<String>,
    pub range: Range,
    pub children: Vec<CodeAST>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start_row: usize,
    pub start_column: usize,
    pub end_row: usize,
    pub end_column: usize,
}

lazy_static! {
    // Language queries for extracting important code structures
    static ref RUST_QUERY: &'static str = r#"
        (struct_item name: (identifier) @struct.name) @struct.def
        (enum_item name: (identifier) @enum.name) @enum.def
        (trait_item name: (identifier) @trait.name) @trait.def
        (impl_item type: (type_identifier) @impl.type) @impl.def
        (function_item name: (identifier) @function.name) @function.def
        (mod_item name: (identifier) @module.name) @module.def
    "#;

    static ref JAVASCRIPT_QUERY: &'static str = r#"
        (class_declaration name: (identifier) @class.name) @class.def
        (function_declaration name: (identifier) @function.name) @function.def
        (method_definition name: (property_identifier) @method.name) @method.def
        (lexical_declaration 
            (variable_declarator 
                name: (identifier) @const.name 
                value: (arrow_function) @const.value)) @const.def
    "#;

    static ref PYTHON_QUERY: &'static str = r#"
        (class_definition name: (identifier) @class.name) @class.def
        (function_definition name: (identifier) @function.name) @function.def
    "#;

    static ref GO_QUERY: &'static str = r#"
        (type_declaration (type_spec name: (type_identifier) @type.name)) @type.def
        (function_declaration name: (identifier) @function.name) @function.def
        (method_declaration name: (field_identifier) @method.name) @method.def
        (struct_type) @struct.def
        (interface_type) @interface.def
    "#;

    // Simple regex fallbacks
    static ref RUST_STRUCT_RE: Regex = Regex::new(r"struct\s+([A-Za-z0-9_]+)").unwrap();
    static ref RUST_ENUM_RE: Regex = Regex::new(r"enum\s+([A-Za-z0-9_]+)").unwrap();
    static ref RUST_IMPL_RE: Regex = Regex::new(r"impl(?:\s+<[^>]+>)?\s+([A-Za-z0-9_:]+)").unwrap();
    static ref RUST_FN_RE: Regex = Regex::new(r"fn\s+([A-Za-z0-9_]+)").unwrap();
    static ref RUST_TRAIT_RE: Regex = Regex::new(r"trait\s+([A-Za-z0-9_]+)").unwrap();
    static ref RUST_MOD_RE: Regex = Regex::new(r"mod\s+([A-Za-z0-9_]+)").unwrap();

    static ref JS_CLASS_RE: Regex = Regex::new(r"class\s+([A-Za-z0-9_]+)").unwrap();
    static ref JS_FUNCTION_RE: Regex = Regex::new(r"function\s+([A-Za-z0-9_]+)").unwrap();
    static ref JS_ARROW_FN_RE: Regex = Regex::new(r"const\s+([A-Za-z0-9_]+)\s*=\s*\([^)]*\)\s*=>").unwrap();
    static ref JS_INTERFACE_RE: Regex = Regex::new(r"interface\s+([A-Za-z0-9_]+)").unwrap();
    static ref JS_TYPE_RE: Regex = Regex::new(r"type\s+([A-Za-z0-9_]+)").unwrap();

    static ref PY_CLASS_RE: Regex = Regex::new(r"class\s+([A-Za-z0-9_]+)").unwrap();
    static ref PY_FUNCTION_RE: Regex = Regex::new(r"def\s+([A-Za-z0-9_]+)").unwrap();
    static ref PY_ASYNC_FN_RE: Regex = Regex::new(r"async\s+def\s+([A-Za-z0-9_]+)").unwrap();

    static ref GENERIC_BLOCK_RE: Regex = Regex::new(r"^\s*[{}]").unwrap();

    // Cache parsers and languages
    static ref LANGUAGE_CACHE: Arc<Mutex<HashMap<String, Language>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref QUERY_CACHE: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub struct CodeParser {
    languages: HashMap<String, Vec<String>>,
    parser: Parser,
}

impl CodeParser {
    pub fn new() -> Result<Self> {
        let mut languages = HashMap::new();

        // Define supported languages (extensible for future needs)
        languages.insert("rust".to_string(), vec!["rs".to_string()]);
        languages.insert(
            "javascript".to_string(),
            vec!["js".to_string(), "jsx".to_string()],
        );
        languages.insert(
            "typescript".to_string(),
            vec!["ts".to_string(), "tsx".to_string()],
        );
        languages.insert("python".to_string(), vec!["py".to_string()]);
        languages.insert("go".to_string(), vec!["go".to_string()]);
        languages.insert("c".to_string(), vec!["c".to_string(), "h".to_string()]);
        languages.insert(
            "cpp".to_string(),
            vec![
                "cpp".to_string(),
                "cc".to_string(),
                "cxx".to_string(),
                "hpp".to_string(),
                "hxx".to_string(),
            ],
        );
        languages.insert("java".to_string(), vec!["java".to_string()]);

        // Initialize parser
        let parser = Parser::new();

        // Initialize language cache with known languages
        {
            let mut cache = LANGUAGE_CACHE.lock().unwrap();
            if cache.is_empty() {
                // Load languages with the tree-sitter bindings
                let rust_lang: Language = tree_sitter_rust::LANGUAGE.into();
                cache.insert("rust".to_string(), rust_lang);

                let js_lang: Language = tree_sitter_javascript::LANGUAGE.into();
                cache.insert("javascript".to_string(), js_lang.clone());
                cache.insert("typescript".to_string(), js_lang); // TypeScript uses JS grammar for basic parsing

                let py_lang: Language = tree_sitter_python::LANGUAGE.into();
                cache.insert("python".to_string(), py_lang);

                let c_lang: Language = tree_sitter_c::LANGUAGE.into();
                cache.insert("c".to_string(), c_lang);

                let cpp_lang: Language = tree_sitter_cpp::LANGUAGE.into();
                cache.insert("cpp".to_string(), cpp_lang);

                let go_lang: Language = tree_sitter_go::LANGUAGE.into();
                cache.insert("go".to_string(), go_lang);

                let java_lang: Language = tree_sitter_java::LANGUAGE.into();
                cache.insert("java".to_string(), java_lang);
            }
        }

        // Initialize query cache with known language queries
        {
            let mut cache = QUERY_CACHE.lock().unwrap();
            if cache.is_empty() {
                cache.insert("rust".to_string(), RUST_QUERY.to_string());
                cache.insert("javascript".to_string(), JAVASCRIPT_QUERY.to_string());
                cache.insert("typescript".to_string(), JAVASCRIPT_QUERY.to_string());
                cache.insert("python".to_string(), PYTHON_QUERY.to_string());
                cache.insert("go".to_string(), GO_QUERY.to_string());
            }
        }

        Ok(Self { languages, parser })
    }

    /// Try to get tree-sitter language for parsing
    fn get_language(&self, language_name: &str) -> Option<Language> {
        let cache = LANGUAGE_CACHE.lock().unwrap();
        cache.get(language_name).cloned()
    }

    /// Try to get query for a language
    fn get_query(&self, language_name: &str) -> Option<Query> {
        let query_cache = QUERY_CACHE.lock().unwrap();
        if let Some(query_string) = query_cache.get(language_name) {
            if let Some(lang) = self.get_language(language_name) {
                return Query::new(&lang, query_string).ok();
            }
        }
        None
    }

    /// Determine language from file extension
    pub fn detect_language(&self, path: &Path) -> Option<String> {
        let extension = path.extension()?.to_str()?.to_lowercase();

        // Special handling for TypeScript/JavaScript
        if extension == "ts" || extension == "tsx" {
            return Some("typescript".to_string());
        } else if extension == "js" || extension == "jsx" {
            return Some("javascript".to_string());
        }

        // General language detection
        for (lang, extensions) in &self.languages {
            if extensions.iter().any(|ext| ext == &extension) {
                return Some(lang.clone());
            }
        }

        None
    }

    /// Parse a single file using tree-sitter and generate AST with size optimizations
    pub fn parse_file(&mut self, path: &Path) -> Result<CodeAST> {
        // Detect language
        let language_name = self
            .detect_language(path)
            .context(format!("Could not detect language for file: {:?}", path))?;

        // Read file content - limit file size for very large files
        let metadata = fs::metadata(path)?;

        // Skip files larger than 1MB to avoid processing too much data
        if metadata.len() > 1_000_000 {
            return Ok(CodeAST {
                path: path.to_string_lossy().to_string(),
                language: language_name.to_string(),
                kind: "file".to_string(),
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                range: Range {
                    start_row: 0,
                    start_column: 0,
                    end_row: 0,
                    end_column: 0,
                },
                children: vec![CodeAST {
                    path: String::new(),
                    language: language_name.to_string(),
                    kind: "large_file".to_string(),
                    name: Some("File too large for AST generation".to_string()),
                    range: Range {
                        start_row: 0,
                        start_column: 0,
                        end_row: 0,
                        end_column: 0,
                    },
                    children: Vec::new(),
                    content: Some(format!(
                        "File size: {} bytes - too large for detailed parsing",
                        metadata.len()
                    )),
                }],
                content: None,
            });
        }

        // Read file content
        let source_code = fs::read_to_string(path)?;

        // Create the base AST node for the file
        let mut ast = CodeAST {
            path: path.to_string_lossy().to_string(),
            language: language_name.to_string(),
            kind: "file".to_string(),
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
            range: Range {
                start_row: 0,
                start_column: 0,
                end_row: source_code.lines().count(),
                end_column: 0,
            },
            children: Vec::new(),
            content: None,
        };

        // Try to use tree-sitter for parsing
        if let Some(language) = self.get_language(&language_name) {
            // Configure parser
            self.parser.set_language(&language)?;

            // Parse the source code
            if let Some(tree) = self.parser.parse(&source_code, None) {
                // Try to use tree-sitter queries to extract structured information
                if let Some(_query) = self.get_query(&language_name) {
                    // Skip tree-sitter query-based parsing for now since we're having compatibility issues
                    // We'll rely on more basic parsing methods instead
                    let root_node = tree.root_node();
                    let root_type = root_node.kind();

                    // Add some basic information about the root node
                    let child_ast = CodeAST {
                        path: String::new(),
                        language: language_name.to_string(),
                        kind: "file_root".to_string(),
                        name: Some(root_type.to_string()),
                        range: Range {
                            start_row: root_node.start_position().row,
                            start_column: root_node.start_position().column,
                            end_row: root_node.end_position().row,
                            end_column: root_node.end_position().column,
                        },
                        children: Vec::new(),
                        content: Some(format!("Root node type: {}", root_type)),
                    };

                    ast.children.push(child_ast);
                }

                // If tree-sitter worked and found structures, return the AST
                if !ast.children.is_empty() {
                    return Ok(ast);
                }

                // If tree-sitter query didn't find anything useful, try traversing the syntax tree
                // Limit to top-level nodes only to reduce size
                let mut node_children =
                    self.extract_important_nodes(tree.root_node(), &source_code, &language_name);

                // Limit to 30 children max
                if node_children.len() > 30 {
                    node_children.truncate(30);
                }

                if !node_children.is_empty() {
                    ast.children = node_children;
                    return Ok(ast);
                }
            }
        }

        // Fallback to simplified regex-based parsing
        // We're optimizing for conciseness, so limit capture size
        self.create_simplified_ast(path, &language_name, &source_code)
    }

    /// Extract important nodes from a tree-sitter syntax tree using generic traversal
    fn extract_important_nodes(
        &self,
        node: Node<'_>,
        source: &str,
        language: &str,
    ) -> Vec<CodeAST> {
        let mut result = Vec::new();
        let important_node_types = match language {
            "rust" => &[
                "struct_item",
                "enum_item",
                "impl_item",
                "function_item",
                "trait_item",
                "mod_item",
                "macro_definition",
            ],
            "javascript" | "typescript" => &[
                "class_declaration",
                "function_declaration",
                "method_definition",
                "lexical_declaration",
                "interface_declaration",
                "export_statement",
                "variable_declaration", // Add an extra item to match the size of the rust array
            ],
            "python" => &[
                "class_definition",
                "function_definition",
                "decorated_definition",
                "import_statement",
                "assignment",
                "expression_statement",
                "return_statement", // Added entries to match array size
            ],
            "go" => &[
                "function_declaration",
                "method_declaration",
                "type_declaration",
                "struct_type",
                "interface_type",
                "package_clause",
                "import_declaration", // Added to match array size
            ],
            "c" | "cpp" => &[
                "function_definition",
                "class_specifier",
                "struct_specifier",
                "enum_specifier",
                "namespace_definition",
                "template_declaration",
                "declaration", // Added to match
            ],
            "java" => &[
                "class_declaration",
                "method_declaration",
                "interface_declaration",
                "constructor_declaration",
                "field_declaration",
                "import_declaration",
                "package_declaration", // Added to match
            ],
            _ => &[
                "unknown", "unknown", "unknown", "unknown", "unknown", "unknown", "unknown",
            ], // Dummy values to match size
        };

        // Check if this node is important
        if important_node_types.contains(&node.kind()) {
            self.process_important_node(node, source, language, &mut result);
        }

        // Recursively process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Skip tokens and trivial nodes
            if child.child_count() > 0 && !child.is_named() {
                let child_results = self.extract_important_nodes(child, source, language);
                result.extend(child_results);
            }
        }

        result
    }

    /// Process an individual node that has been identified as important - optimized for size
    fn process_important_node(
        &self,
        node: Node<'_>,
        source: &str,
        language: &str,
        result: &mut Vec<CodeAST>,
    ) {
        // Try to find a name for this node
        let mut name = None;
        let mut cursor = node.walk();

        // Look for identifier nodes that might contain the name
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier"
                || child.kind() == "type_identifier"
                || child.kind() == "field_identifier"
                || child.kind() == "property_identifier"
            {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    name = Some(text.to_string());
                    break;
                }
            }
        }

        // Extract just the first line of content as a preview
        let content = node
            .utf8_text(source.as_bytes())
            .ok()
            .and_then(|s| s.lines().next())
            .map(|first_line| {
                // Limit content length
                if first_line.len() > 100 {
                    format!("{}...", &first_line[..100])
                } else {
                    first_line.to_string()
                }
            });

        // Create a minimal AST node for this important node
        let ast_node = CodeAST {
            path: String::new(),
            language: language.to_string(),
            kind: node.kind().to_string(),
            name,
            range: Range {
                start_row: node.start_position().row,
                start_column: 0, // Skip column info to save space
                end_row: node.end_position().row,
                end_column: 0, // Skip column info to save space
            },
            children: Vec::new(),
            content,
        };

        result.push(ast_node);
    }

    /// Fallback method: Create a simplified AST using regex - optimized for size
    pub fn create_simplified_ast(
        &self,
        path: &Path,
        language: &str,
        source_code: &str,
    ) -> Result<CodeAST> {
        // Limit input size for regex processing
        let limited_source = if source_code.len() > 50_000 {
            // Only process first ~50KB to avoid regex performance issues
            let truncated: String = source_code.chars().take(50_000).collect();
            truncated
        } else {
            source_code.to_string()
        };

        let lines: Vec<&str> = limited_source.lines().collect();

        // Create basic AST structure
        let mut ast = CodeAST {
            path: path.to_string_lossy().to_string(),
            language: language.to_string(),
            kind: "file".to_string(),
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
            range: Range {
                start_row: 0,
                start_column: 0,
                end_row: lines.len(),
                end_column: 0, // Skip end column to save space
            },
            children: Vec::new(),
            content: None,
        };

        // Extract top-level structures based on language, limit to most relevant ones
        let mut children = match language {
            "rust" => self.extract_rust_constructs(&limited_source),
            "javascript" | "typescript" => self.extract_js_constructs(&limited_source),
            "python" => self.extract_python_constructs(&limited_source),
            _ => self.extract_generic_constructs(&limited_source),
        };

        // Limit number of children to reduce overall size
        if children.len() > 30 {
            children.truncate(30);
        }

        ast.children = children;

        Ok(ast)
    }

    // Helper to create a minimal AST node from a regex match
    fn create_ast_node(&self, params: AstNodeParams) -> CodeAST {
        CodeAST {
            path: String::new(), // Not relevant for child nodes
            language: params.language.to_string(),
            kind: params.kind.to_string(),
            name: Some(params.capture.to_string()),
            range: Range {
                start_row: params.line_num,
                start_column: params.match_start, // Use match column info
                end_row: params.line_num,
                end_column: params.match_end, // Use match column info
            },
            children: Vec::new(),
            // Only include a short preview of the line
            content: if params.line.len() > 100 {
                Some(format!("{}...", &params.line[..100]))
            } else {
                Some(params.line.to_string())
            },
        }
    }

    // Extract Rust constructs using regex (fallback method)
    fn extract_rust_constructs(&self, source: &str) -> Vec<CodeAST> {
        let mut constructs = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Process each line to find Rust constructs
        for (line_num, line) in lines.iter().enumerate() {
            // Check for structs
            if let Some(captures) = RUST_STRUCT_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "rust",
                        kind: "struct",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for enums
            if let Some(captures) = RUST_ENUM_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "rust",
                        kind: "enum",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for impls
            if let Some(captures) = RUST_IMPL_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "rust",
                        kind: "impl",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for functions
            if let Some(captures) = RUST_FN_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "rust",
                        kind: "function",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for traits
            if let Some(captures) = RUST_TRAIT_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "rust",
                        kind: "trait",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for modules
            if let Some(captures) = RUST_MOD_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "rust",
                        kind: "module",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }
        }

        constructs
    }

    // Extract JavaScript/TypeScript constructs using regex (fallback method)
    fn extract_js_constructs(&self, source: &str) -> Vec<CodeAST> {
        let mut constructs = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Process each line to find JS/TS constructs
        for (line_num, line) in lines.iter().enumerate() {
            // Check for classes
            if let Some(captures) = JS_CLASS_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "javascript",
                        kind: "class",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for functions
            if let Some(captures) = JS_FUNCTION_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "javascript",
                        kind: "function",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for arrow functions
            if let Some(captures) = JS_ARROW_FN_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "javascript",
                        kind: "arrow_function",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for interfaces (TypeScript)
            if let Some(captures) = JS_INTERFACE_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "javascript",
                        kind: "interface",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for types (TypeScript)
            if let Some(captures) = JS_TYPE_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "javascript",
                        kind: "type",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }
        }

        constructs
    }

    // Extract Python constructs using regex (fallback method)
    fn extract_python_constructs(&self, source: &str) -> Vec<CodeAST> {
        let mut constructs = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Process each line to find Python constructs
        for (line_num, line) in lines.iter().enumerate() {
            // Check for classes
            if let Some(captures) = PY_CLASS_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "python",
                        kind: "class",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for functions
            if let Some(captures) = PY_FUNCTION_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "python",
                        kind: "function",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }

            // Check for async functions
            if let Some(captures) = PY_ASYNC_FN_RE.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    constructs.push(self.create_ast_node(AstNodeParams {
                        language: "python",
                        kind: "async_function",
                        line_num,
                        line,
                        capture: name_match.as_str(),
                        match_start: name_match.start(),
                        match_end: name_match.end(),
                    }));
                }
            }
        }

        constructs
    }

    // Extract generic code constructs (fallback method)
    fn extract_generic_constructs(&self, source: &str) -> Vec<CodeAST> {
        let mut constructs = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Process each line to find generic code blocks
        for (line_num, line) in lines.iter().enumerate() {
            if GENERIC_BLOCK_RE.is_match(line) {
                constructs.push(CodeAST {
                    path: String::new(),
                    language: "generic".to_string(),
                    kind: "block".to_string(),
                    name: None,
                    range: Range {
                        start_row: line_num,
                        start_column: 0,
                        end_row: line_num,
                        end_column: line.len(),
                    },
                    children: Vec::new(),
                    content: Some(line.to_string()),
                });
            }
        }

        constructs
    }

    /// Use search tools to find relevant files for a query, with efficiency optimizations
    fn find_relevant_files(&self, root_dir: &Path, query: &str) -> Result<Vec<PathBuf>> {
        use crate::tools::fs::search::SearchTools;

        let mut results = Vec::new();

        // Hard limit on number of files to process
        let max_files = 25; // Reduced from 50 to limit AST size

        // Filter to respect gitignore patterns using the ignore crate
        let filter_gitignore = |path: &Path| -> bool {
            // Create a walker that respects gitignore
            let walker = WalkBuilder::new(path)
                .hidden(false) // Include hidden files
                .git_ignore(true) // Respect gitignore
                .build();

            // If the walker yields this path, it's not ignored
            walker.flatten().any(|entry| entry.path() == path)
        };

        // Start with more targeted approach - look for specific files first
        // Extract specific file mentions from query (like "check file.rs" or "in models.rs")
        let file_regex =
            Regex::new(r"(?:file|in|check|view|read)\s+([a-zA-Z0-9_\-\.]+\.[a-zA-Z0-9]+)").unwrap();
        let mut specific_files = Vec::new();

        for cap in file_regex.captures_iter(query) {
            if let Some(file_name) = cap.get(1) {
                specific_files.push(format!("**/{}", file_name.as_str()));
            }
        }

        // If specific files were mentioned, prioritize those
        if !specific_files.is_empty() {
            for pattern in &specific_files {
                if let Ok(matches) = SearchTools::glob_search(pattern) {
                    for path in matches {
                        if !results.contains(&path) && filter_gitignore(&path) {
                            results.push(path);
                            if results.len() >= max_files {
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // If specific terms were extracted, try grepping for them
        let search_terms = self.extract_search_terms(query);
        if !search_terms.is_empty() {
            // Limit to top few most specific terms
            let top_terms: Vec<String> = search_terms.into_iter().take(3).collect();

            for term in top_terms {
                if let Ok(grep_matches) = SearchTools::grep_search(&term, None, Some(root_dir)) {
                    // Take only top matches
                    for (path, _, _) in grep_matches.into_iter().take(5) {
                        if !results.contains(&path) && filter_gitignore(&path) {
                            results.push(path);
                            if results.len() >= max_files {
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // If we still need more files, use patterns based on query content
        if results.len() < max_files {
            // Get a smaller set of more targeted patterns
            let patterns = self.determine_relevant_files(query);
            let targeted_patterns: Vec<&String> = patterns.iter().take(5).collect();

            for pattern in targeted_patterns {
                if let Ok(matches) = SearchTools::glob_search(pattern) {
                    for path in matches.into_iter().take(5) {
                        if !results.contains(&path) && filter_gitignore(&path) {
                            results.push(path);
                            if results.len() >= max_files {
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // If still not enough, add a few key project files
        if results.len() < 5 {
            let key_project_files = vec![
                "**/lib.rs",
                "**/main.rs",
                "**/mod.rs",
                "**/Cargo.toml",
                "**/package.json",
                "**/README.md",
            ];

            for pattern in key_project_files {
                if let Ok(matches) = SearchTools::glob_search(pattern) {
                    for path in matches {
                        if !results.contains(&path) && filter_gitignore(&path) {
                            results.push(path);
                            if results.len() >= max_files {
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        // Sort results by modification time to prioritize recently changed files
        results.sort_by(|a, b| {
            let a_modified = std::fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_modified = std::fs::metadata(b).and_then(|m| m.modified()).ok();
            b_modified.cmp(&a_modified)
        });

        Ok(results)
    }

    /// Extract search terms from a query for grep search
    pub fn extract_search_terms(&self, query: &str) -> Vec<String> {
        let mut terms = Vec::new();

        // Split query into words and look for potential code identifiers
        let words: Vec<&str> = query
            .split_whitespace()
            .filter(|w| w.len() > 3) // Skip short words
            .collect();

        for word in words {
            // Clean up the word to extract potential identifiers
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');

            // Look for identifiers that match coding convention patterns
            if clean_word.len() > 3
                && clean_word.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !clean_word.chars().all(|c| c.is_numeric())
            {
                // Skip common English words and programming keywords
                let common_words = [
                    "this",
                    "that",
                    "from",
                    "what",
                    "when",
                    "where",
                    "which",
                    "find",
                    "function",
                    "class",
                    "struct",
                    "impl",
                    "type",
                    "interface",
                    "const",
                    "static",
                    "public",
                    "private",
                    "protected",
                    "export",
                    "import",
                ];

                if !common_words.contains(&clean_word.to_lowercase().as_str()) {
                    terms.push(clean_word.to_string());
                }
            }
        }

        terms
    }

    /// Parse an entire codebase and generate ASTs for selected files
    pub fn parse_codebase(&mut self, root_dir: &Path, query: &str) -> Result<Vec<CodeAST>> {
        let mut asts = Vec::new();

        // Get files relevant to the query
        let relevant_files = self.find_relevant_files(root_dir, query)?;

        // Parse each file
        for path in relevant_files {
            if let Ok(ast) = self.parse_file(&path) {
                asts.push(ast);
            }
        }

        Ok(asts)
    }

    /// Generate a concise AST optimized for LLM consumption, respecting API size limits
    pub fn generate_llm_friendly_ast(&mut self, root_dir: &Path, query: &str) -> Result<String> {
        // Parse the relevant parts of the codebase
        let mut asts = self.parse_codebase(root_dir, query)?;

        // If no AST data was generated, return a helpful message
        if asts.is_empty() {
            return Ok(String::from("No relevant code structures found for the query. Try to be more specific about what code you're looking for."));
        }

        // Sort ASTs by relevance (assuming more recently modified files are more relevant)
        asts.sort_by(|a, b| {
            let a_path = Path::new(&a.path);
            let b_path = Path::new(&b.path);

            let a_modified = std::fs::metadata(a_path).and_then(|m| m.modified()).ok();
            let b_modified = std::fs::metadata(b_path).and_then(|m| m.modified()).ok();

            b_modified.cmp(&a_modified)
        });

        // Limit to most relevant files (10 max)
        if asts.len() > 10 {
            asts.truncate(10);
        }

        // Limit content size within each AST node
        for ast in &mut asts {
            // Limit child nodes to most important ones (max 20 per file)
            if ast.children.len() > 20 {
                ast.children.truncate(20);
            }

            // Truncate content for each child node
            for child in &mut ast.children {
                if let Some(content) = &child.content {
                    if content.len() > 500 {
                        let truncated: String = content.chars().take(500).collect();
                        child.content = Some(format!("{}... [truncated]", truncated));
                    }
                }
            }
        }

        // Generate a summary of the AST data
        let mut summary = String::new();
        summary.push_str(&format!(
            "# Code Structure Analysis for Query: \"{}\"\n\n",
            query
        ));
        summary.push_str(&format!(
            "Found {} relevant files (showing {} most relevant). Key structures:\n\n",
            asts.len(),
            asts.len()
        ));

        // Add a simple text summary of the most important structures
        for ast in &asts {
            summary.push_str(&format!("## File: {}\n", ast.path));
            summary.push_str(&format!("Language: {}\n\n", ast.language));

            for child in &ast.children {
                let name = child.name.as_deref().unwrap_or("anonymous");
                summary.push_str(&format!(
                    "- {} `{}` at line {}\n",
                    child.kind,
                    name,
                    child.range.start_row + 1
                ));

                // Include a short snippet of the content if available
                if let Some(content) = &child.content {
                    // Only take first line for brevity
                    let first_line = content.lines().next().unwrap_or("");
                    if !first_line.is_empty() {
                        summary.push_str(&format!(
                            "   ```{}\n   {}\n   ```\n",
                            ast.language, first_line
                        ));
                    }
                }
            }

            summary.push('\n');
        }

        // Create a simplified JSON representation with just the essential information
        let simplified_asts: Vec<serde_json::Value> = asts
            .iter()
            .map(|ast| {
                let simplified_children: Vec<serde_json::Value> = ast
                    .children
                    .iter()
                    .map(|child| {
                        serde_json::json!({
                            "kind": child.kind,
                            "name": child.name,
                            "line": child.range.start_row + 1
                        })
                    })
                    .collect();

                serde_json::json!({
                    "path": ast.path,
                    "language": ast.language,
                    "entities": simplified_children
                })
            })
            .collect();

        // Add the simplified JSON representation
        summary.push_str("\n## Simplified Code Structure:\n\n```json\n");
        let simplified_json = serde_json::to_string_pretty(&simplified_asts)
            .context("Failed to serialize simplified AST to JSON")?;
        summary.push_str(&simplified_json);
        summary.push_str("\n```\n");

        // Add full AST data in JSON format for more detailed analysis
        summary.push_str("\n## Full AST Data (JSON):\n\n```json\n");
        let full_json =
            serde_json::to_string_pretty(&asts).context("Failed to serialize full AST to JSON")?;
        summary.push_str(&full_json);
        summary.push_str("\n```\n");

        Ok(summary)
    }

    /// Determine which files to parse based on user query
    pub fn determine_relevant_files(&self, query: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Look for specific file mentions in the query
        let file_regex = Regex::new(r#"['"]([^'"]+\.\w+)['"]"#).unwrap();
        for cap in file_regex.captures_iter(query) {
            if let Some(file_match) = cap.get(1) {
                let file_pattern = format!("**/{}", file_match.as_str());
                patterns.push(file_pattern);
            }
        }

        // Add language-specific patterns based on query keywords
        let query_lower = query.to_lowercase();

        // Rust patterns
        if query_lower.contains("rust") || query_lower.contains(".rs") {
            patterns.push("**/*.rs".to_string());
            patterns.push("**/src/**/*.rs".to_string());
            patterns.push("**/lib.rs".to_string());
            patterns.push("**/main.rs".to_string());
        }

        // JavaScript patterns
        if query_lower.contains("javascript")
            || query_lower.contains("js")
            || query_lower.contains("node")
            || query_lower.contains("react")
        {
            patterns.push("**/*.js".to_string());
            patterns.push("**/*.jsx".to_string());
            patterns.push("**/src/**/*.js".to_string());
            patterns.push("**/src/**/*.jsx".to_string());
        }

        // TypeScript patterns
        if query_lower.contains("typescript")
            || query_lower.contains("ts")
            || query_lower.contains("angular")
            || query_lower.contains("next")
        {
            patterns.push("**/*.ts".to_string());
            patterns.push("**/*.tsx".to_string());
            patterns.push("**/src/**/*.ts".to_string());
            patterns.push("**/src/**/*.tsx".to_string());
        }

        // Python patterns
        if query_lower.contains("python")
            || query_lower.contains("py")
            || query_lower.contains("django")
            || query_lower.contains("flask")
        {
            patterns.push("**/*.py".to_string());
            patterns.push("**/src/**/*.py".to_string());
        }

        // Go patterns
        if query_lower.contains("go") || query_lower.contains("golang") {
            patterns.push("**/*.go".to_string());
            patterns.push("**/src/**/*.go".to_string());
        }

        // C/C++ patterns
        if query_lower.contains("c++")
            || query_lower.contains("cpp")
            || query_lower.contains(" c ")
            || query_lower.contains(".c")
        {
            patterns.push("**/*.c".to_string());
            patterns.push("**/*.h".to_string());
            patterns.push("**/*.cpp".to_string());
            patterns.push("**/*.hpp".to_string());
            patterns.push("**/*.cc".to_string());
        }

        // Java patterns
        if query_lower.contains("java") && !query_lower.contains("javascript") {
            patterns.push("**/*.java".to_string());
            patterns.push("**/src/**/*.java".to_string());
        }

        // Add patterns for common code directories if no specific language mentioned
        if patterns.is_empty() || !patterns.iter().any(|p| p.starts_with("**/src/")) {
            patterns.push("**/src/**/*.rs".to_string());
            patterns.push("**/src/**/*.ts".to_string());
            patterns.push("**/src/**/*.js".to_string());
            patterns.push("**/src/**/*.py".to_string());
        }

        // Always add the language of the codebase (assuming Rust for oli)
        if !patterns.iter().any(|p| p.ends_with(".rs")) {
            patterns.push("**/*.rs".to_string());
        }

        patterns
    }
}
