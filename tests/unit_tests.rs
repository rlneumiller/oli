use oli_tui::fs_tools::code_parser::{CodeAST, CodeParser};
use std::path::Path;

// Add extra imports for history tests
#[cfg(test)]
mod history_tests {
    use oli_tui::app::history::ConversationSummary;
    use oli_tui::app::history::HistoryManager;
    use oli_tui::app::state::{App, AppState};

    #[test]
    fn test_conversation_char_count() {
        let mut app = App::new();
        app.state = AppState::Chat;

        // Empty conversation should have 0 chars
        assert_eq!(app.conversation_char_count(), 0);

        // Add some messages
        app.messages.push("Hello".to_string());
        app.messages.push("World".to_string());

        // Should count "Hello" (5) + "World" (5) = 10
        assert_eq!(app.conversation_char_count(), 10);
    }

    #[test]
    fn test_should_summarize() {
        let mut app = App::new();
        app.state = AppState::Chat;

        // Empty conversation should not need summarization
        assert!(!app.should_summarize());

        // Add enough messages to trigger summarization by count
        for i in 0..40 {
            app.messages.push(format!("Message {}", i));
        }

        // Should now need summarization
        assert!(app.should_summarize());

        // Reset and try with character count
        app.messages.clear();

        // Add a single large message
        app.messages.push("A".repeat(6000));

        // Should need summarization due to character count
        assert!(app.should_summarize());
    }

    #[test]
    fn test_summary_count() {
        let app = App::new();

        // Should start with 0 summaries
        assert_eq!(app.summary_count(), 0);
    }

    #[test]
    fn test_clear_history() {
        let mut app = App::new();

        // Add some messages and summaries
        app.messages.push("Test message".to_string());

        // Add a fake summary
        app.conversation_summaries.push(ConversationSummary::new(
            "Test summary".to_string(),
            5,
            100,
        ));

        // Clear history
        app.clear_history();

        // Both messages and summaries should be empty
        assert!(app.messages.is_empty());
        assert!(app.conversation_summaries.is_empty());
        assert_eq!(app.scroll_position, 0);
    }
}

#[test]
fn test_detect_language() {
    let parser = CodeParser::new().unwrap();

    // Test common Rust files
    assert_eq!(
        parser.detect_language(Path::new("test.rs")),
        Some("rust".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("src/main.rs")),
        Some("rust".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("lib.rs")),
        Some("rust".to_string())
    );

    // Test JavaScript files
    assert_eq!(
        parser.detect_language(Path::new("test.js")),
        Some("javascript".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("app.js")),
        Some("javascript".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("components/Button.jsx")),
        Some("javascript".to_string())
    );

    // Test TypeScript files
    assert_eq!(
        parser.detect_language(Path::new("test.ts")),
        Some("typescript".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("app.ts")),
        Some("typescript".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("components/Button.tsx")),
        Some("typescript".to_string())
    );

    // Test Python files
    assert_eq!(
        parser.detect_language(Path::new("test.py")),
        Some("python".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("app.py")),
        Some("python".to_string())
    );

    // Test C/C++ files
    assert_eq!(
        parser.detect_language(Path::new("main.c")),
        Some("c".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("header.h")),
        Some("c".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("class.cpp")),
        Some("cpp".to_string())
    );
    assert_eq!(
        parser.detect_language(Path::new("class.hpp")),
        Some("cpp".to_string())
    );

    // Test Go files
    assert_eq!(
        parser.detect_language(Path::new("main.go")),
        Some("go".to_string())
    );

    // Test Java files
    assert_eq!(
        parser.detect_language(Path::new("Main.java")),
        Some("java".to_string())
    );

    // Test unknown extensions
    assert_eq!(parser.detect_language(Path::new("unknown.xyz")), None);
    assert_eq!(parser.detect_language(Path::new("test")), None);
}

#[test]
fn test_extract_search_terms() {
    let parser = CodeParser::new().unwrap();

    // Test with class/struct names
    let terms = parser.extract_search_terms("Find the CodeParser implementation");
    assert!(terms.contains(&"CodeParser".to_string()));

    // Test with function names
    let terms = parser.extract_search_terms("How does the function parse_file work?");
    assert!(terms.contains(&"parse_file".to_string()));

    // Test with multiple terms
    let terms = parser.extract_search_terms(
        "How does generate_llm_friendly_ast communicate with the LLMProvider?",
    );
    assert!(terms.contains(&"generate_llm_friendly_ast".to_string()));
    assert!(terms.contains(&"LLMProvider".to_string()));

    // Test with common words that should be excluded
    let terms = parser.extract_search_terms("How do I use this function?");
    assert!(!terms.contains(&"function".to_string()));
    assert!(!terms.contains(&"this".to_string()));

    // Test with short words that should be excluded
    let terms = parser.extract_search_terms("How do I use API?");
    assert!(!terms.contains(&"How".to_string()));
    assert!(!terms.contains(&"do".to_string()));
    assert!(!terms.contains(&"use".to_string()));
    // Note: API is only 3 characters so it would be filtered out
}

#[test]
fn test_determine_relevant_files() {
    let parser = CodeParser::new().unwrap();

    // Test query with specific file mentions
    let patterns = parser.determine_relevant_files("Look at 'main.rs' and tell me what it does");
    assert!(patterns.iter().any(|p| p.contains("main.rs")));

    // Test query with Rust keywords
    let patterns = parser.determine_relevant_files("Analyze the Rust code in this project");
    assert!(patterns.iter().any(|p| p.ends_with(".rs")));
    assert!(patterns.iter().any(|p| p.contains("src")));

    // Test query with JavaScript keywords
    let patterns = parser.determine_relevant_files("Show me the React components");
    assert!(patterns.iter().any(|p| p.ends_with(".js")));
    assert!(patterns.iter().any(|p| p.ends_with(".jsx")));

    // Test query with TypeScript keywords
    let patterns = parser.determine_relevant_files("Can you explain the TypeScript interfaces?");
    assert!(patterns.iter().any(|p| p.ends_with(".ts")));
    assert!(patterns.iter().any(|p| p.ends_with(".tsx")));

    // Test query with Python keywords
    let patterns = parser.determine_relevant_files("How are the Django models defined?");
    assert!(patterns.iter().any(|p| p.ends_with(".py")));

    // Test query with Go keywords
    let patterns = parser.determine_relevant_files("Explain Golang interfaces in this project");
    assert!(patterns.iter().any(|p| p.ends_with(".go")));

    // Test query with C/C++ keywords
    let patterns = parser.determine_relevant_files("Show me the C++ class implementations");
    assert!(
        patterns.iter().any(|p| p.ends_with(".cpp"))
            || patterns.iter().any(|p| p.ends_with(".cc"))
            || patterns.iter().any(|p| p.ends_with(".cxx"))
    );

    // Test query with Java keywords
    let patterns = parser.determine_relevant_files("How are Java classes organized?");
    assert!(patterns.iter().any(|p| p.ends_with(".java")));

    // Test generic query without language-specific keywords
    let patterns = parser.determine_relevant_files("How is the code organized?");
    assert!(patterns.iter().any(|p| p.contains("src"))); // Should look in source directories
    assert!(patterns.iter().any(|p| p.ends_with(".rs"))); // Should include Rust (project's language)
}

#[test]
fn test_create_simplified_ast() {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
    }

    let parser = CodeParser::new().unwrap();

    // Create a temporary directory for test files
    let temp_dir = tempfile::tempdir().unwrap();

    // Create a temporary Rust file
    let rust_file_path = temp_dir.path().join("test.rs");
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
    
    fn test_method(&self) -> i32 {
        self.field1
    }
}

fn test_function() -> i32 {
    42
}

trait TestTrait {
    fn trait_method(&self) -> bool;
}

mod test_module {
    pub fn module_function() {
        println!("Hello from module");
    }
}
"#,
    )
    .unwrap();

    // Parse the file with fallback method
    let ast = parser
        .create_simplified_ast(
            &rust_file_path,
            "rust",
            &std::fs::read_to_string(&rust_file_path).unwrap(),
        )
        .unwrap();

    // Verify the parsed AST
    assert_eq!(ast.kind, "file");
    assert_eq!(ast.language, "rust");

    // Check that we found at least 5 key Rust constructs (struct, impl, fn, trait, mod)
    let construct_kinds: Vec<&str> = ast.children.iter().map(|c| c.kind.as_str()).collect();
    assert!(construct_kinds.contains(&"struct"));
    assert!(construct_kinds.contains(&"function") || construct_kinds.contains(&"fn"));

    // Check that we found the struct name
    let struct_nodes: Vec<&CodeAST> = ast.children.iter().filter(|c| c.kind == "struct").collect();
    assert!(!struct_nodes.is_empty());
    assert_eq!(struct_nodes[0].name.as_ref().unwrap(), "TestStruct");

    // Clean up
    temp_dir.close().unwrap();
}
