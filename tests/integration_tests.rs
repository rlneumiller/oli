use oli_tui::fs_tools::code_parser::CodeParser;

#[test]
fn test_generate_llm_friendly_ast() {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
    }

    // Create a temporary directory for test files
    let temp_dir = tempfile::tempdir().unwrap();

    // Create a simple Rust file for testing
    let rust_file_path = temp_dir.path().join("test.rs");
    std::fs::write(
        &rust_file_path,
        r#"
struct TestStruct {
    field1: i32,
    field2: String,
}

fn main() {
    println!("Hello, world!");
}
"#,
    )
    .unwrap();

    // Initialize parser
    let mut parser = CodeParser::new().expect("Failed to create CodeParser");

    // Create a targeted query that focuses only on our test file
    let query = "Show me the TestStruct implementation";

    // Generate AST based on the query, using the temp directory as root
    let ast_data = parser
        .generate_llm_friendly_ast(temp_dir.path(), query)
        .expect("Failed to generate AST");

    // Verify that the AST contains meaningful content
    assert!(ast_data.contains("Code Structure Analysis"));
    assert!(ast_data.contains("Query:"));

    // Test file should be found
    assert!(ast_data.contains("test.rs"));
    assert!(ast_data.contains("TestStruct"));

    // Verify JSON data is included
    assert!(ast_data.contains("Full AST Data (JSON):"));
    assert!(ast_data.contains("```json"));

    // Clean up
    temp_dir.close().unwrap();
}

#[test]
fn test_parse_codebase() {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
    }

    // Create a temporary directory for test files
    let temp_dir = tempfile::tempdir().unwrap();

    // Create a simple Rust file with CodeParser-like content
    let rust_file_path = temp_dir.path().join("code_parser_test.rs");
    std::fs::write(
        &rust_file_path,
        r#"
struct CodeParser {
    languages: HashMap<String, Vec<String>>,
    parser: Parser,
}

impl CodeParser {
    pub fn new() -> Result<Self> {
        Ok(Self { 
            languages: HashMap::new(),
            parser: Parser::new(),
        })
    }
    
    pub fn parse_file(&mut self, path: &Path) -> Result<CodeAST> {
        // Implementation...
        todo!()
    }
}
"#,
    )
    .unwrap();

    // Initialize parser
    let mut parser = CodeParser::new().expect("Failed to create CodeParser");

    // Create a test query that should find our test file
    let query = "Show me the CodeParser implementation";

    // Parse the temp directory
    let asts = parser
        .parse_codebase(temp_dir.path(), query)
        .expect("Failed to parse codebase");

    // We should find at least one file
    assert!(!asts.is_empty(), "Expected to find at least one file");

    // Check that at least one AST entry matches our expected file
    let found_code_parser = asts.iter().any(|ast| {
        ast.path.contains("code_parser_test.rs")
            || (ast.language == "rust"
                && ast.children.iter().any(|child| {
                    child
                        .name
                        .as_ref()
                        .is_some_and(|name| name.contains("CodeParser"))
                }))
    });

    assert!(
        found_code_parser,
        "Did not find CodeParser in the parsed AST"
    );

    // Clean up
    temp_dir.close().unwrap();
}

#[test]
fn test_parse_file_with_tree_sitter() {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
    }

    // Create a temporary directory for test files
    let temp_dir = tempfile::tempdir().unwrap();

    // Create a simple Rust file for testing
    let rust_file_path = temp_dir.path().join("test_tree_sitter.rs");
    std::fs::write(
        &rust_file_path,
        r#"
struct TestStruct {
    field1: i32,
    field2: String,
}

impl TestStruct {
    fn new() -> Self {
        Self {
            field1: 0,
            field2: String::new(),
        }
    }
}

fn main() {
    let test = TestStruct::new();
    println!("Test program");
}
"#,
    )
    .unwrap();

    // Initialize parser
    let mut parser = CodeParser::new().expect("Failed to create CodeParser");

    // Parse the file
    let ast = parser
        .parse_file(&rust_file_path)
        .expect("Failed to parse file");

    // Verify that we got a valid AST
    assert_eq!(ast.kind, "file");
    assert_eq!(ast.language, "rust");
    assert!(!ast.children.is_empty(), "Expected to find AST nodes");

    // Check for key Rust elements
    let struct_nodes: Vec<_> = ast
        .children
        .iter()
        .filter(|node| node.kind == "struct" || node.kind == "struct_item")
        .collect();

    let function_nodes: Vec<_> = ast
        .children
        .iter()
        .filter(|node| node.kind == "function" || node.kind == "function_item")
        .collect();

    // We should have found either via tree-sitter query or the regex fallback
    assert!(
        !struct_nodes.is_empty() || !function_nodes.is_empty(),
        "Expected to find either struct or function nodes"
    );

    // Clean up
    temp_dir.close().unwrap();
}
