use oli_tui::tools::code::parser::CodeParser;
use std::fs;
use tempfile::tempdir;

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

    // Verify the new AST Summary format is used
    assert!(ast_data.contains("AST Summary"));
    assert!(ast_data.contains("Analyzed"));

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

#[test]
fn test_parse_javascript_file() {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
    }

    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.js");

    // Create test JavaScript file with various language constructs
    fs::write(
        &file_path,
        r#"
class TestClass {
    constructor(value) {
        this.value = value;
    }

    getValue() {
        return this.value;
    }
}

function testFunction() {
    return "test";
}

const arrowFunc = () => {
    return "arrow";
};

const obj = {
    method() {
        return this;
    }
};

export const exportedVar = 42;
        "#,
    )
    .unwrap();

    let mut parser = CodeParser::new().unwrap();
    let ast = parser.parse_file(&file_path).unwrap();

    // Basic AST validation
    assert_eq!(ast.language, "javascript");
    assert_eq!(ast.kind, "file");

    // Verify it found our key structures (either with tree-sitter specific names or fallback names)
    let child_kinds: Vec<_> = ast.children.iter().map(|c| c.kind.as_str()).collect();
    assert!(
        child_kinds.contains(&"class") || child_kinds.contains(&"class_declaration"),
        "Expected to find class declarations"
    );

    // Clean up
    temp_dir.close().unwrap();
}

#[test]
fn test_extract_search_terms() {
    let parser = CodeParser::new().unwrap();

    // Test extraction of code identifiers
    let terms = parser.extract_search_terms("Find the TestStruct implementation");
    assert!(terms.contains(&"TestStruct".to_string()));

    // Test with function names
    let terms = parser.extract_search_terms("How does the parse_file function work?");
    assert!(terms.contains(&"parse_file".to_string()));

    // Test with multiple terms
    let terms = parser.extract_search_terms("Show me how CodeParser uses tree_sitter");
    assert!(terms.contains(&"CodeParser".to_string()));
    assert!(terms.contains(&"tree_sitter".to_string()));

    // Test that common words are filtered
    let terms = parser.extract_search_terms("Show me the function and class definitions");
    assert!(!terms.contains(&"function".to_string()));
    assert!(!terms.contains(&"class".to_string()));
}

#[test]
fn test_determine_relevant_files() {
    let parser = CodeParser::new().unwrap();

    // Test with file mentions
    let patterns = parser.determine_relevant_files("Check 'main.rs' for the entry point");
    assert!(patterns.iter().any(|p| p.contains("main.rs")));

    // Test language-specific patterns
    let patterns = parser.determine_relevant_files("Show me the Rust code structure");
    assert!(patterns.iter().any(|p| p.ends_with(".rs")));

    let patterns = parser.determine_relevant_files("Analyze JavaScript components");
    assert!(
        patterns.iter().any(|p| p.ends_with(".js")) || patterns.iter().any(|p| p.ends_with(".jsx"))
    );

    let patterns = parser.determine_relevant_files("Parse Python classes");
    assert!(patterns.iter().any(|p| p.ends_with(".py")));
}

#[test]
fn test_parallel_processing() {
    // Skip if SKIP_INTEGRATION is set
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
    }

    let temp_dir = tempdir().unwrap();

    // Create multiple sample files
    let file1_path = temp_dir.path().join("file1.rs");
    fs::write(&file1_path, "struct File1 { field: i32 }").unwrap();

    let file2_path = temp_dir.path().join("file2.rs");
    fs::write(&file2_path, "struct File2 { field: i32 }").unwrap();

    let file3_path = temp_dir.path().join("file3.rs");
    fs::write(&file3_path, "struct File3 { field: i32 }").unwrap();

    // Create subdirectory with a file
    let subdir_path = temp_dir.path().join("subdir");
    fs::create_dir(&subdir_path).unwrap();
    let file4_path = subdir_path.join("file4.rs");
    fs::write(&file4_path, "struct File4 { field: i32 }").unwrap();

    let mut parser = CodeParser::new().unwrap();
    let query = "Find all structs";

    // This test primarily verifies that parallel processing doesn't crash
    // The actual number of files found might vary based on implementation details
    let asts = parser.parse_codebase(temp_dir.path(), query).unwrap();

    // We should find at least some files
    assert!(!asts.is_empty(), "Expected to find at least some files");

    // Clean up
    temp_dir.close().unwrap();
}
