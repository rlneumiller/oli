use anyhow::{Context, Result};
use ignore::WalkBuilder;
use lazy_static::lazy_static;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator, Tree};

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

/// Represents a source code location range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start_row: usize,
    pub start_column: usize,
    pub end_row: usize,
    pub end_column: usize,
}

// Global query definitions for tree-sitter parsing
lazy_static! {
    /// PEG-style query for Rust code structures
    static ref RUST_QUERY: &'static str = r#"
        ; Struct declarations
        (struct_item
            name: (identifier) @struct.name
            body: (field_declaration_list)? @struct.body) @struct.def

        ; Enum declarations
        (enum_item
            name: (identifier) @enum.name
            body: (enum_variant_list)? @enum.body) @enum.def

        ; Trait declarations
        (trait_item
            name: (identifier) @trait.name
            body: (declaration_list)? @trait.body) @trait.def

        ; Implementations
        (impl_item
            trait: (type_identifier)? @impl.trait
            type: (type_identifier) @impl.type
            body: (declaration_list)? @impl.body) @impl.def

        ; Functions
        (function_item
            name: (identifier) @function.name
            parameters: (parameters)? @function.params
            body: (block)? @function.body) @function.def

        ; Modules
        (mod_item
            name: (identifier) @module.name
            body: (declaration_list)? @module.body) @module.def

        ; Constants and statics
        (const_item
            name: (identifier) @const.name
            type: (_) @const.type
            value: (_) @const.value) @const.def

        (static_item
            name: (identifier) @static.name
            type: (_) @static.type
            value: (_) @static.value) @static.def
    "#;

    /// PEG-style query for JavaScript/TypeScript code structures
    static ref JAVASCRIPT_QUERY: &'static str = r#"
        ; Classes
        (class_declaration
            name: (identifier) @class.name
            body: (class_body)? @class.body) @class.def

        ; Functions
        (function_declaration
            name: (identifier) @function.name
            parameters: (formal_parameters) @function.params
            body: (statement_block)? @function.body) @function.def

        ; Methods
        (method_definition
            name: (property_identifier) @method.name
            parameters: (formal_parameters) @method.params
            body: (statement_block)? @method.body) @method.def

        ; Arrow functions in variable declarations
        (lexical_declaration
            (variable_declarator
                name: (identifier) @const.name
                value: (arrow_function) @const.value)) @const.def

        ; Object pattern in variable declarations
        (variable_declaration
            (variable_declarator
                name: (identifier) @var.name)) @var.def

        ; Interface declarations (TypeScript)
        (interface_declaration
            name: (type_identifier) @interface.name
            body: (object_type)? @interface.body) @interface.def

        ; Type aliases (TypeScript)
        (type_alias_declaration
            name: (type_identifier) @type.name
            value: (_) @type.value) @type.def

        ; Export declarations
        (export_statement
            declaration: (_) @export.declaration) @export.def
    "#;

    /// PEG-style query for Python code structures
    static ref PYTHON_QUERY: &'static str = r#"
        ; Classes
        (class_definition
            name: (identifier) @class.name
            body: (block)? @class.body) @class.def

        ; Functions
        (function_definition
            name: (identifier) @function.name
            parameters: (parameters) @function.params
            body: (block)? @function.body) @function.def

        ; Decorated definitions
        (decorated_definition
            definition: (_) @decorated.definition) @decorated.def

        ; Imports
        (import_statement
            name: (dotted_name) @import.name) @import.def

        (import_from_statement
            module_name: (dotted_name) @import_from.module) @import_from.def

        ; Global variables and constants
        (assignment
            left: (identifier) @assignment.name
            right: (_) @assignment.value) @assignment.def

        ; Class attributes
        (class_definition
            body: (block
                (expression_statement
                    (assignment
                        left: (identifier) @class_attr.name)))) @class_attr.def
    "#;

    /// PEG-style query for Go code structures
    static ref GO_QUERY: &'static str = r#"
        ; Type declarations
        (type_declaration
            (type_spec
                name: (type_identifier) @type.name
                type: (_) @type.value)) @type.def

        ; Function declarations
        (function_declaration
            name: (identifier) @function.name
            parameters: (parameter_list) @function.params
            result: (_)? @function.result
            body: (block)? @function.body) @function.def

        ; Method declarations
        (method_declaration
            name: (field_identifier) @method.name
            parameters: (parameter_list) @method.params
            result: (_)? @method.result
            body: (block)? @method.body) @method.def

        ; Struct type definitions
        (type_declaration
            (type_spec
                name: (type_identifier) @struct.name
                type: (struct_type) @struct.body)) @struct.def

        ; Interface type definitions
        (type_declaration
            (type_spec
                name: (type_identifier) @interface.name
                type: (interface_type) @interface.body)) @interface.def

        ; Package clause
        (package_clause
            (package_identifier) @package.name) @package.def

        ; Import declarations
        (import_declaration
            (import_spec_list) @import.specs) @import.def
    "#;

    /// PEG-style query for C/C++ code structures
    static ref CPP_QUERY: &'static str = r#"
        ; Function definitions
        (function_definition
            declarator: (function_declarator
                declarator: (identifier) @function.name
                parameters: (parameter_list) @function.params)
            body: (compound_statement) @function.body) @function.def

        ; Class specifiers
        (class_specifier
            name: (type_identifier) @class.name
            body: (field_declaration_list) @class.body) @class.def

        ; Struct specifiers
        (struct_specifier
            name: (type_identifier) @struct.name
            body: (field_declaration_list) @struct.body) @struct.def

        ; Enum specifiers
        (enum_specifier
            name: (type_identifier) @enum.name
            body: (enumerator_list) @enum.body) @enum.def

        ; Namespace definitions
        (namespace_definition
            name: (identifier) @namespace.name
            body: (declaration_list) @namespace.body) @namespace.def

        ; Template declarations
        (template_declaration
            parameters: (template_parameter_list) @template.params
            declaration: (_) @template.declaration) @template.def

        ; Variable declarations
        (declaration
            declarator: (init_declarator
                declarator: (identifier) @var.name)) @var.def

        ; Method definitions
        (function_definition
            declarator: (function_declarator
                declarator: (field_identifier) @method.name
                parameters: (parameter_list) @method.params)
            body: (compound_statement) @method.body) @method.def
    "#;

    /// PEG-style query for Java code structures
    static ref JAVA_QUERY: &'static str = r#"
        ; Class declarations
        (class_declaration
            name: (identifier) @class.name
            body: (class_body) @class.body) @class.def

        ; Method declarations
        (method_declaration
            name: (identifier) @method.name
            parameters: (formal_parameters) @method.params
            body: (block)? @method.body) @method.def

        ; Interface declarations
        (interface_declaration
            name: (identifier) @interface.name
            body: (interface_body) @interface.body) @interface.def

        ; Constructor declarations
        (constructor_declaration
            name: (identifier) @constructor.name
            parameters: (formal_parameters) @constructor.params
            body: (constructor_body) @constructor.body) @constructor.def

        ; Field declarations
        (field_declaration
            declarator: (variable_declarator
                name: (identifier) @field.name)) @field.def

        ; Package declarations
        (package_declaration
            name: (scoped_identifier) @package.name) @package.def

        ; Import declarations
        (import_declaration
            name: (scoped_identifier) @import.name) @import.def

        ; Annotation declarations
        (annotation_type_declaration
            name: (identifier) @annotation.name
            body: (annotation_type_body) @annotation.body) @annotation.def
    "#;

    // Cache for parsers and languages
    static ref LANGUAGE_CACHE: Arc<RwLock<HashMap<String, Language>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref PARSER_CACHE: Arc<Mutex<HashMap<String, Parser>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref QUERY_CACHE: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref TREE_CACHE: Arc<RwLock<HashMap<PathBuf, (Tree, String)>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// A robust code parser system that analyzes source code and produces
/// clean, accurate Abstract Syntax Trees (ASTs) optimized for LLM consumption.
///
/// # Key capabilities:
/// - Consistent parsing approach using tree-sitter for reliable, accurate parsing
/// - Clean, well-documented API for LLM tool use
/// - Efficient error recovery for handling malformed code
/// - Structured AST output that LLMs can easily interpret
/// - Language detection with robust extension mapping
/// - Declarative query patterns for extracting meaningful code structures
/// - Efficient caching system for parsers and queries
pub struct CodeParser {
    /// Maps language names to file extensions
    languages: HashMap<String, Vec<String>>,
    /// Default parser instance for initial parsing
    parser: Parser,
    /// Cache size limit for AST trees (in bytes)
    cache_size_limit: usize,
    /// Maximum file size to parse in bytes (default: 1MB)
    max_file_size: usize,
    /// Maximum number of files to parse in a codebase (default: 25)
    max_files: usize,
    /// Maximum recursion depth for nested structures (default: 3)
    max_depth: usize,
}

impl CodeParser {
    /// Creates a new CodeParser instance with initialized language support
    /// and default configuration.
    ///
    /// # Returns
    /// - `Result<Self>` - A new CodeParser instance or an error
    pub fn new() -> Result<Self> {
        Self::with_config(None, None, None, None)
    }

    /// Creates a new CodeParser instance with custom configuration.
    ///
    /// # Arguments
    /// - `cache_size_limit` - Optional cache size limit in bytes (default: 50MB)
    /// - `max_file_size` - Optional maximum file size to parse in bytes (default: 1MB)
    /// - `max_files` - Optional maximum number of files to parse (default: 25)
    /// - `max_depth` - Optional maximum recursion depth (default: 3)
    ///
    /// # Returns
    /// - `Result<Self>` - A new CodeParser instance or an error
    pub fn with_config(
        cache_size_limit: Option<usize>,
        max_file_size: Option<usize>,
        max_files: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<Self> {
        let mut languages = HashMap::new();

        // Define supported languages with their file extensions
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
            let mut cache = LANGUAGE_CACHE.write().unwrap();
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

        // Initialize parser cache
        {
            let mut cache = PARSER_CACHE.lock().unwrap();
            if cache.is_empty() {
                for lang_name in languages.keys() {
                    let mut new_parser = Parser::new();
                    if let Some(lang) = LANGUAGE_CACHE.read().unwrap().get(lang_name) {
                        if new_parser.set_language(lang).is_ok() {
                            cache.insert(lang_name.clone(), new_parser);
                        }
                    }
                }
            }
        }

        // Initialize query cache with known language queries
        {
            let mut cache = QUERY_CACHE.write().unwrap();
            if cache.is_empty() {
                cache.insert("rust".to_string(), RUST_QUERY.to_string());
                cache.insert("javascript".to_string(), JAVASCRIPT_QUERY.to_string());
                cache.insert("typescript".to_string(), JAVASCRIPT_QUERY.to_string());
                cache.insert("python".to_string(), PYTHON_QUERY.to_string());
                cache.insert("go".to_string(), GO_QUERY.to_string());
                cache.insert("c".to_string(), CPP_QUERY.to_string());
                cache.insert("cpp".to_string(), CPP_QUERY.to_string());
                cache.insert("java".to_string(), JAVA_QUERY.to_string());
            }
        }

        // Set defaults or use provided values
        let cache_size_limit = cache_size_limit.unwrap_or(50 * 1024 * 1024); // 50MB cache
        let max_file_size = max_file_size.unwrap_or(1_000_000); // 1MB max file size
        let max_files = max_files.unwrap_or(25); // Maximum files to parse
        let max_depth = max_depth.unwrap_or(3); // Maximum recursion depth

        Ok(Self {
            languages,
            parser,
            cache_size_limit,
            max_file_size,
            max_files,
            max_depth,
        })
    }

    /// Gets a tree-sitter language for parsing
    ///
    /// # Arguments
    /// - `language_name` - Name of the language to retrieve
    ///
    /// # Returns
    /// - `Option<Language>` - The tree-sitter language if available
    fn get_language(&self, language_name: &str) -> Option<Language> {
        LANGUAGE_CACHE.read().unwrap().get(language_name).cloned()
    }

    // Note: get_parser method removed as it was unused

    /// Gets a tree-sitter query for a language
    ///
    /// # Arguments
    /// - `language_name` - Name of the language to get a query for
    ///
    /// # Returns
    /// - `Option<Query>` - A tree-sitter query if available
    fn get_query(&self, language_name: &str) -> Option<Result<Query>> {
        let query_cache = QUERY_CACHE.read().unwrap();
        let query_string = query_cache.get(language_name)?;

        if let Some(lang) = self.get_language(language_name) {
            match Query::new(&lang, query_string) {
                Ok(query) => Some(Ok(query)),
                Err(e) => Some(Err(anyhow::anyhow!("Failed to create query: {:?}", e))),
            }
        } else {
            None
        }
    }

    /// Determines the programming language from a file extension
    ///
    /// # Arguments
    /// - `path` - Path to the file
    ///
    /// # Returns
    /// - `Option<String>` - Language name if detected
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

    /// Parses a single file using tree-sitter and generates an AST
    ///
    /// # Arguments
    /// - `path` - Path to the file to parse
    ///
    /// # Returns
    /// - `Result<CodeAST>` - The abstract syntax tree or an error
    pub fn parse_file(&mut self, path: &Path) -> Result<CodeAST> {
        // Detect language
        let language_name = self
            .detect_language(path)
            .context(format!("Could not detect language for file: {:?}", path))?;

        // Read file content - limit file size for very large files
        let metadata = fs::metadata(path)?;

        // Skip files larger than the max file size to avoid processing too much data
        if metadata.len() > self.max_file_size as u64 {
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
            // Check tree cache first
            let path_buf = path.to_path_buf();
            let tree_option = {
                let cache = TREE_CACHE.read().unwrap();
                if let Some((tree, content)) = cache.get(&path_buf) {
                    if content == &source_code {
                        Some(tree.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            // If tree is not in cache, parse it
            let tree = if let Some(cached_tree) = tree_option {
                cached_tree
            } else {
                // Configure parser
                self.parser.set_language(&language)?;

                // Parse with error recovery
                let tree = self
                    .parser
                    .parse(&source_code, None)
                    .context("Failed to parse source code")?;

                // Store in cache
                {
                    let mut cache = TREE_CACHE.write().unwrap();

                    // Check if we need to evict some entries to stay within the cache size limit
                    let current_size: usize =
                        cache.iter().map(|(_, (_, content))| content.len()).sum();

                    if current_size + source_code.len() > self.cache_size_limit {
                        // Simple LRU eviction: remove oldest entries first
                        let mut keys_to_remove = Vec::new();
                        let mut entries: Vec<_> = cache.iter().collect();
                        entries.sort_by_key(|(_, (_, content))| content.len());

                        let mut freed_size = 0;
                        let needed_size = source_code.len();

                        for (path, (_, content)) in entries {
                            if current_size + needed_size - freed_size <= self.cache_size_limit {
                                break;
                            }

                            freed_size += content.len();
                            keys_to_remove.push(path.clone());
                        }

                        // Remove entries after we're done iterating
                        for path in keys_to_remove {
                            cache.remove(&path);
                        }
                    }

                    cache.insert(path_buf.clone(), (tree.clone(), source_code.clone()));
                }

                tree
            };

            // Try to use tree-sitter queries to extract structured information
            if let Some(Ok(query)) = self.get_query(&language_name) {
                // Use tree-sitter query to extract structured information
                let root_node = tree.root_node();
                let mut query_cursor = QueryCursor::new();

                // Extract nodes based on the query
                let mut matches = query_cursor.matches(&query, root_node, source_code.as_bytes());

                // Process matches to extract AST nodes
                while let Some(match_item) = matches.next() {
                    let mut node_data: HashMap<String, (Node, String)> = HashMap::new();

                    // Extract each capture data
                    for capture in match_item.captures {
                        // Get the capture name
                        let capture_name = &query.capture_names()[capture.index as usize];
                        let node_text = capture
                            .node
                            .utf8_text(source_code.as_bytes())
                            .unwrap_or("<unknown>");

                        node_data.insert(
                            capture_name.to_string(),
                            (capture.node, node_text.to_string()),
                        );
                    }

                    // Find definition nodes (with .def suffix)
                    let def_entries: Vec<_> = node_data
                        .iter()
                        .filter(|(name, _)| name.ends_with(".def"))
                        .collect();

                    if !def_entries.is_empty() {
                        let (def_name, (def_node, _)) = def_entries[0];
                        let def_type = def_name.split('.').next().unwrap_or("unknown");
                        let start_pos = def_node.start_position();
                        let end_pos = def_node.end_position();

                        // Find name nodes (with .name suffix)
                        let name_entries: Vec<_> = node_data
                            .iter()
                            .filter(|(name, _)| name.ends_with(".name"))
                            .collect();

                        let name = if !name_entries.is_empty() {
                            let (_, (_, name_text)) = name_entries[0];
                            Some(name_text.clone())
                        } else {
                            None
                        };

                        // Extract body content if available
                        let body_entries: Vec<_> = node_data
                            .iter()
                            .filter(|(name, _)| name.ends_with(".body"))
                            .collect();

                        let content = if !body_entries.is_empty() {
                            let (_, (body_node, _)) = body_entries[0];
                            body_node
                                .utf8_text(source_code.as_bytes())
                                .ok()
                                .map(|s| s.to_string())
                        } else {
                            def_node
                                .utf8_text(source_code.as_bytes())
                                .ok()
                                .map(|s| s.to_string())
                        };

                        // Create child AST node
                        let mut child_ast = CodeAST {
                            path: String::new(),
                            language: language_name.to_string(),
                            kind: def_type.to_string(),
                            name,
                            range: Range {
                                start_row: start_pos.row,
                                start_column: start_pos.column,
                                end_row: end_pos.row,
                                end_column: end_pos.column,
                            },
                            children: Vec::new(),
                            content: content.map(|s| {
                                // Truncate content if it's too large
                                if s.len() > 1000 {
                                    format!("{}...", &s[..1000])
                                } else {
                                    s
                                }
                            }),
                        };

                        // Extract nested structures (for hierarchical AST)
                        self.extract_nested_structures(
                            &source_code,
                            *def_node,
                            &mut child_ast,
                            &language_name,
                            self.max_depth, // Use configured recursion depth
                        );

                        ast.children.push(child_ast);
                    }
                }

                // If tree-sitter query found structures, return the AST
                if !ast.children.is_empty() {
                    return Ok(ast);
                }
            }

            // If structured query didn't work, fallback to node traversal
            let tree_node = tree.root_node();
            let node_children =
                self.extract_important_nodes(tree_node, &source_code, &language_name);

            if !node_children.is_empty() {
                ast.children = node_children;
                return Ok(ast);
            }
        }

        // If tree-sitter couldn't produce useful results, use simplified extraction
        self.create_simplified_ast(path, &language_name, &source_code)
    }

    /// Extracts nested structures from a node to build a hierarchical AST
    ///
    /// # Arguments
    /// - `source_code` - Source code of the file
    /// - `node` - Current node to process
    /// - `parent_ast` - Parent AST node to add children to
    /// - `language` - Language of the source code
    /// - `depth` - Recursion depth limit
    fn extract_nested_structures(
        &self,
        source_code: &str,
        node: Node,
        parent_ast: &mut CodeAST,
        language: &str,
        depth: usize,
    ) {
        if depth == 0 {
            return;
        }

        // Skip if the node is too small
        if node.end_byte() - node.start_byte() < 10 {
            return;
        }

        let mut cursor = node.walk();

        // Get nested defined structures based on language
        let important_node_types = Self::get_important_node_types(language);

        // Process child nodes
        for child in node.children(&mut cursor) {
            let kind = child.kind();

            // Skip insignificant nodes
            if kind == "(" || kind == ")" || kind == "{" || kind == "}" || kind == ";" {
                continue;
            }

            // Process important child nodes
            if important_node_types.contains(&kind) {
                let start_pos = child.start_position();
                let end_pos = child.end_position();

                // Try to find a name for this node
                let name = self.extract_node_name(&child, source_code);

                // Get content truncated for brevity
                let content = child.utf8_text(source_code.as_bytes()).ok().map(|s| {
                    // Truncate content if it's too large
                    if s.len() > 500 {
                        format!("{}...", &s[..500])
                    } else {
                        s.to_string()
                    }
                });

                // Create child AST node
                let mut child_ast = CodeAST {
                    path: String::new(),
                    language: language.to_string(),
                    kind: kind.to_string(),
                    name,
                    range: Range {
                        start_row: start_pos.row,
                        start_column: start_pos.column,
                        end_row: end_pos.row,
                        end_column: end_pos.column,
                    },
                    children: Vec::new(),
                    content,
                };

                // Recursively extract nested structures
                self.extract_nested_structures(
                    source_code,
                    child,
                    &mut child_ast,
                    language,
                    depth - 1,
                );

                parent_ast.children.push(child_ast);
            }
        }
    }

    /// Extracts a name from a node based on common patterns
    ///
    /// # Arguments
    /// - `node` - Node to extract name from
    /// - `source` - Source code
    ///
    /// # Returns
    /// - `Option<String>` - Extracted name if found
    fn extract_node_name(&self, node: &Node, source: &str) -> Option<String> {
        let mut cursor = node.walk();

        // Look for identifier nodes that might contain the name
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier"
                || child.kind() == "type_identifier"
                || child.kind() == "field_identifier"
                || child.kind() == "property_identifier"
            {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Some(text.to_string());
                }
            }
        }

        None
    }

    /// Gets a list of important node types for a given language
    ///
    /// # Arguments
    /// - `language` - Language to get node types for
    ///
    /// # Returns
    /// - `&[&str]` - Array of important node type names
    fn get_important_node_types(language: &str) -> &'static [&'static str] {
        match language {
            "rust" => &[
                "struct_item",
                "enum_item",
                "impl_item",
                "function_item",
                "trait_item",
                "mod_item",
                "macro_definition",
                "const_item",
                "static_item",
            ],
            "javascript" | "typescript" => &[
                "class_declaration",
                "function_declaration",
                "method_definition",
                "lexical_declaration",
                "interface_declaration",
                "type_alias_declaration",
                "export_statement",
                "variable_declaration",
            ],
            "python" => &[
                "class_definition",
                "function_definition",
                "decorated_definition",
                "import_statement",
                "import_from_statement",
                "assignment",
            ],
            "go" => &[
                "function_declaration",
                "method_declaration",
                "type_declaration",
                "struct_type",
                "interface_type",
                "package_clause",
                "import_declaration",
            ],
            "c" | "cpp" => &[
                "function_definition",
                "class_specifier",
                "struct_specifier",
                "enum_specifier",
                "namespace_definition",
                "template_declaration",
                "declaration",
            ],
            "java" => &[
                "class_declaration",
                "method_declaration",
                "interface_declaration",
                "constructor_declaration",
                "field_declaration",
                "package_declaration",
                "import_declaration",
                "annotation_type_declaration",
            ],
            _ => &[],
        }
    }

    /// Extract important nodes from a tree-sitter syntax tree using generic traversal
    ///
    /// # Arguments
    /// - `node` - Root node to traverse
    /// - `source` - Source code text
    /// - `language` - Language of the source code
    ///
    /// # Returns
    /// - `Vec<CodeAST>` - List of extracted AST nodes
    fn extract_important_nodes(
        &self,
        node: Node<'_>,
        source: &str,
        language: &str,
    ) -> Vec<CodeAST> {
        let mut result = Vec::new();
        let important_node_types = Self::get_important_node_types(language);

        // Check if this node is important
        if important_node_types.contains(&node.kind()) {
            self.process_important_node(node, source, language, &mut result);
        }

        // Recursively process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Skip tokens and trivial nodes
            if child.child_count() > 0 && child.is_named() {
                let child_results = self.extract_important_nodes(child, source, language);
                result.extend(child_results);
            }
        }

        result
    }

    /// Process an individual node that has been identified as important
    ///
    /// # Arguments
    /// - `node` - Node to process
    /// - `source` - Source code text
    /// - `language` - Language of the source code
    /// - `result` - Vector to add processed nodes to
    fn process_important_node(
        &self,
        node: Node<'_>,
        source: &str,
        language: &str,
        result: &mut Vec<CodeAST>,
    ) {
        // Try to find a name for this node
        let name = self.extract_node_name(&node, source);

        // Extract content (full node text for better context)
        let content = node.utf8_text(source.as_bytes()).ok().map(|s| {
            // Truncate content if it's too large
            if s.len() > 500 {
                format!("{}...", &s[..500])
            } else {
                s.to_string()
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
                start_column: node.start_position().column,
                end_row: node.end_position().row,
                end_column: node.end_position().column,
            },
            children: Vec::new(),
            content,
        };

        result.push(ast_node);
    }

    /// Create a simplified AST directly from the source code
    /// This is a fallback method when tree-sitter parsing doesn't work
    ///
    /// # Arguments
    /// - `path` - Path to the file
    /// - `language` - Language of the source code
    /// - `source_code` - Source code text
    ///
    /// # Returns
    /// - `Result<CodeAST>` - Simplified AST or error
    pub fn create_simplified_ast(
        &self,
        path: &Path,
        language: &str,
        source_code: &str,
    ) -> Result<CodeAST> {
        // Limit input size for processing
        let limited_source = if source_code.len() > 50_000 {
            // Only process first ~50KB for efficiency
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
                end_column: 0,
            },
            children: Vec::new(),
            content: None,
        };

        // Extract code blocks and declarations with line numbers
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and simple statements
            if trimmed.is_empty() || (trimmed.len() < 5 && !trimmed.contains('{')) {
                continue;
            }

            // Identify potential code blocks and declarations by common patterns
            if trimmed.contains(" fn ")
                || trimmed.contains("func ")
                || trimmed.contains(" class ")
                || trimmed.contains(" struct ")
                || trimmed.contains(" trait ")
                || trimmed.contains(" impl ")
                || trimmed.contains(" interface ")
                || trimmed.contains(" def ")
                || trimmed.contains(" type ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("interface ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("async ")
            {
                // Determine the kind of node
                let kind = if trimmed.contains(" fn ")
                    || trimmed.contains("func ")
                    || trimmed.starts_with("fn ")
                    || trimmed.contains(" def ")
                    || trimmed.starts_with("def ")
                    || trimmed.starts_with("function ")
                    || trimmed.contains("async ")
                {
                    "function"
                } else if trimmed.contains(" class ") || trimmed.starts_with("class ") {
                    "class"
                } else if trimmed.contains(" struct ") || trimmed.starts_with("struct ") {
                    "struct"
                } else if trimmed.contains(" trait ") || trimmed.starts_with("trait ") {
                    "trait"
                } else if trimmed.contains(" impl ") || trimmed.starts_with("impl ") {
                    "impl"
                } else if trimmed.contains(" interface ") || trimmed.starts_with("interface ") {
                    "interface"
                } else if trimmed.contains(" type ") || trimmed.starts_with("type ") {
                    "type"
                } else {
                    "block"
                };

                // Extract a simple name from the line by splitting on spaces and symbols
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                let mut name = None;

                // Try to find a name based on the kind (position after keyword)
                if words.len() > 1 {
                    let name_word_idx = match kind {
                        "function" => {
                            if trimmed.contains(" fn ") {
                                words.iter().position(|&w| w == "fn").map(|p| p + 1)
                            } else if trimmed.contains(" def ") {
                                words.iter().position(|&w| w == "def").map(|p| p + 1)
                            } else if trimmed.contains("func ") {
                                words.iter().position(|&w| w == "func").map(|p| p + 1)
                            } else if trimmed.contains(" function ") {
                                words.iter().position(|&w| w == "function").map(|p| p + 1)
                            } else {
                                Some(1) // Assume name is the second word
                            }
                        }
                        "class" => words.iter().position(|&w| w == "class").map(|p| p + 1),
                        "struct" => words.iter().position(|&w| w == "struct").map(|p| p + 1),
                        "trait" => words.iter().position(|&w| w == "trait").map(|p| p + 1),
                        "impl" => words.iter().position(|&w| w == "impl").map(|p| p + 1),
                        "interface" => words.iter().position(|&w| w == "interface").map(|p| p + 1),
                        "type" => words.iter().position(|&w| w == "type").map(|p| p + 1),
                        _ => Some(1),
                    };

                    if let Some(idx) = name_word_idx {
                        if idx < words.len() {
                            // Clean up the name (remove trailing colons, brackets, etc.)
                            name = Some(
                                words[idx]
                                    .trim_end_matches(|c| ",:;<>(){}".contains(c))
                                    .to_string(),
                            );
                        }
                    }
                }

                // Create AST node for this code construct
                let ast_node = CodeAST {
                    path: String::new(),
                    language: language.to_string(),
                    kind: kind.to_string(),
                    name,
                    range: Range {
                        start_row: line_num,
                        start_column: 0,
                        end_row: line_num,
                        end_column: line.len(),
                    },
                    children: Vec::new(),
                    content: Some(line.to_string()),
                };

                ast.children.push(ast_node);
            }
        }

        // Limit number of children to reduce overall size
        if ast.children.len() > 30 {
            ast.children.truncate(30);
        }

        Ok(ast)
    }

    /// Use search tools to find relevant files for a query
    ///
    /// # Arguments
    /// - `root_dir` - Root directory to search in
    /// - `query` - User query to determine relevant files
    ///
    /// # Returns
    /// - `Result<Vec<PathBuf>>` - List of relevant file paths
    fn find_relevant_files(&self, root_dir: &Path, query: &str) -> Result<Vec<PathBuf>> {
        use crate::tools::fs::search::SearchTools;

        let mut results = Vec::new();

        // Use configured limit on number of files to process
        let max_files = self.max_files;

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
            regex::Regex::new(r"(?:file|in|check|view|read)\s+([a-zA-Z0-9_\-\.]+\.[a-zA-Z0-9]+)")
                .unwrap();
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
    ///
    /// # Arguments
    /// - `query` - User query string
    ///
    /// # Returns
    /// - `Vec<String>` - Extracted search terms
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
    ///
    /// # Arguments
    /// - `root_dir` - Root directory of the codebase
    /// - `query` - User query to determine relevant files
    ///
    /// # Returns
    /// - `Result<Vec<CodeAST>>` - List of ASTs for relevant files
    pub fn parse_codebase(&mut self, root_dir: &Path, query: &str) -> Result<Vec<CodeAST>> {
        // Get files relevant to the query
        let relevant_files = self.find_relevant_files(root_dir, query)?;

        // Use parallel processing for better performance
        let asts: Vec<Result<CodeAST>> = relevant_files
            .par_iter()
            .map(|path| {
                let mut local_parser = CodeParser::new()?;
                local_parser.parse_file(path)
            })
            .collect();

        // Filter out errors and collect successful ASTs
        let valid_asts: Vec<CodeAST> = asts
            .into_iter()
            .filter_map(|ast_result| {
                // Just silently ignore parse errors since we're doing best-effort parsing
                // and may not need all files
                ast_result.ok()
            })
            .collect();

        Ok(valid_asts)
    }

    /// Generate a structured AST optimized for LLM consumption
    ///
    /// # Arguments
    /// - `root_dir` - Root directory of the codebase or path to a single file
    /// - `query` - User query to determine relevant files
    ///
    /// # Returns
    /// - `Result<String>` - Structured AST as a string
    pub fn generate_llm_friendly_ast(&mut self, root_dir: &Path, query: &str) -> Result<String> {
        // Check if the path is a file or directory
        let mut asts = if root_dir.is_file() {
            // Just parse this single file
            let ast = self.parse_file(root_dir)?;
            vec![ast]
        } else {
            // Parse the relevant parts of the codebase
            self.parse_codebase(root_dir, query)?
        };

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

        // Create a structured code map that shows the hierarchy of code
        let mut structured_output = String::new();
        structured_output.push_str(&format!(
            "# Code Structure Analysis for Query: \"{}\"

",
            query
        ));

        // Add a hierarchical breakdown of each file
        structured_output.push_str(&format!(
            "## Codebase Structure Overview

{} relevant files found. Showing hierarchical breakdown:

",
            asts.len()
        ));

        // Create a structured code map
        for ast in &asts {
            // Add file header
            structured_output.push_str(&format!("### File: {}\n", ast.path));
            structured_output.push_str(&format!("Language: {}\n\n", ast.language));

            // Sort children by line number for logical ordering
            let mut ordered_children = ast.children.clone();
            ordered_children.sort_by_key(|child| child.range.start_row);

            // Track seen types to avoid duplication in the output
            let mut seen_types = HashSet::new();

            // Add each code structure with line numbers
            for child in &ordered_children {
                let name = child.name.as_deref().unwrap_or("anonymous");

                // Skip if we've already seen this exact type+name combination
                let type_name_key = format!("{}:{}", child.kind, name);
                if seen_types.contains(&type_name_key) {
                    continue;
                }
                seen_types.insert(type_name_key);

                structured_output.push_str(&format!(
                    "- {} `{}` (line {})\n",
                    child.kind,
                    name,
                    child.range.start_row + 1
                ));

                // Add a code snippet if available
                if let Some(content) = &child.content {
                    // Get just the first line or a limited preview
                    let preview = content.lines().next().unwrap_or("");
                    if !preview.is_empty() {
                        structured_output
                            .push_str(&format!("  ```{}\n  {}\n  ```\n", ast.language, preview));
                    }
                }

                // Add nested children if any (for hierarchical display)
                if !child.children.is_empty() {
                    for nested_child in &child.children {
                        if let Some(nested_name) = &nested_child.name {
                            structured_output.push_str(&format!(
                                "  - {} `{}` (line {})\n",
                                nested_child.kind,
                                nested_name,
                                nested_child.range.start_row + 1
                            ));
                        }
                    }
                }
            }

            structured_output.push('\n');
        }

        // Add a table of all identified symbols across files
        structured_output.push_str("## Symbol Table\n\n");
        structured_output.push_str("| Type | Name | File | Line |\n");
        structured_output.push_str("|------|------|------|------|\n");

        // Collect all symbols for the table
        let mut all_symbols = Vec::new();
        for ast in &asts {
            for child in &ast.children {
                if let Some(name) = &child.name {
                    // Skip symbols with generic or empty names
                    if name == "anonymous" || name.is_empty() {
                        continue;
                    }

                    all_symbols.push((
                        child.kind.clone(),
                        name.clone(),
                        ast.path.clone(),
                        child.range.start_row + 1,
                    ));
                }
            }
        }

        // Sort symbols by type and name
        all_symbols.sort_by(|a, b| {
            let type_cmp = a.0.cmp(&b.0);
            if type_cmp == std::cmp::Ordering::Equal {
                a.1.cmp(&b.1)
            } else {
                type_cmp
            }
        });

        // Add symbols to table
        for (kind, name, file, line) in all_symbols {
            // Extract just the file name for brevity
            let file_name = Path::new(&file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            structured_output.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                kind, name, file_name, line
            ));
        }

        // Add a section for relationships between symbols
        structured_output.push_str("\n## Symbol Relationships\n\n");
        structured_output.push_str("This section shows relationships between code elements:\n\n");

        // Extract relationships from the AST (like inheritance, implementation, etc.)
        let mut relationships = Vec::new();

        for ast in &asts {
            // For Rust, look for impl blocks
            if ast.language == "rust" {
                for child in &ast.children {
                    if child.kind == "impl" {
                        if let Some(name) = &child.name {
                            relationships.push(format!(
                                "- `{}` implements trait/functionality for type `{}`",
                                ast.path, name
                            ));
                        }
                    }
                }
            }

            // For other languages, look for inheritance/implementation patterns
            // (This would be expanded based on language-specific patterns)
        }

        if !relationships.is_empty() {
            for relationship in relationships {
                structured_output.push_str(&format!("{}\n", relationship));
            }
        } else {
            structured_output.push_str("No clear relationships detected between symbols.\n");
        }

        // Add the full AST data in JSON format for programmatic use
        // This is limited to avoid overwhelming the LLM with too much data
        structured_output.push_str("\n## AST Summary\n\n");
        // Instead of full JSON, provide a summary of what's available
        structured_output.push_str(&format!(
            "Analyzed {} files containing {} total code structures.\n",
            asts.len(),
            asts.iter().map(|ast| ast.children.len()).sum::<usize>()
        ));

        Ok(structured_output)
    }

    /// Determine which files to parse based on user query
    ///
    /// # Arguments
    /// - `query` - User query string
    ///
    /// # Returns
    /// - `Vec<String>` - List of glob patterns for relevant files
    pub fn determine_relevant_files(&self, query: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Look for specific file mentions in the query
        let file_regex = regex::Regex::new(r#"['"](\S+\.\w+)['"]"#).unwrap();
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
