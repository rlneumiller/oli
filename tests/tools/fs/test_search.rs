use anyhow::Result;
use oli_server::tools::fs::search::SearchTools;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

fn setup_test_directory() -> Result<TempDir> {
    let temp_dir = tempfile::tempdir()?;

    // Create subdirectories for a realistic file structure
    fs::create_dir(temp_dir.path().join("src"))?;
    fs::create_dir(temp_dir.path().join("tests"))?;
    fs::create_dir(temp_dir.path().join("src/models"))?;
    fs::create_dir(temp_dir.path().join("src/utils"))?;

    // Create Rust files with content
    let main_rs = r#"
fn main() {
    println!("Hello, world!");
    let config = load_config();
    initialize_app(config);
}

fn load_config() -> Config {
    // Load configuration from file
    Config::default()
}

fn initialize_app(config: Config) {
    // Initialize application with config
    println!("App initialized with: {:?}", config);
}

struct Config {
    debug: bool,
    port: u16,
}

impl Config {
    fn default() -> Self {
        Self {
            debug: false,
            port: 8080,
        }
    }
}
"#;

    let models_rs = r#"
pub struct User {
    id: u64,
    username: String,
    email: String,
}

impl User {
    pub fn new(id: u64, username: String, email: String) -> Self {
        Self { id, username, email }
    }

    pub fn validate(&self) -> bool {
        !self.username.is_empty() && !self.email.is_empty() && self.email.contains('@')
    }
}

pub struct Post {
    id: u64,
    title: String,
    content: String,
    user_id: u64,
}

impl Post {
    pub fn new(id: u64, title: String, content: String, user_id: u64) -> Self {
        Self { id, title, content, user_id }
    }
}
"#;

    let utils_rs = r#"
pub fn format_date(timestamp: u64) -> String {
    // Format timestamp as date string
    format!("2023-01-01")
}

pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

pub fn generate_id() -> u64 {
    42 // Chosen by fair dice roll, guaranteed to be random
}
"#;

    let test_rs = r#"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com"));
        assert!(!validate_email("invalid-email"));
    }

    #[test]
    fn test_format_date() {
        assert_eq!(format_date(1672531200), "2023-01-01");
    }
}
"#;

    // Create JavaScript file for testing file type filtering
    let js_file = r#"
function validateEmail(email) {
    return email.includes('@') && email.includes('.');
}

class User {
    constructor(id, username, email) {
        this.id = id;
        this.username = username;
        this.email = email;
    }

    validate() {
        return this.username && this.email && validateEmail(this.email);
    }
}

export { User, validateEmail };
"#;

    // Write files to temp directory
    write_file(temp_dir.path().join("src/main.rs"), main_rs)?;
    write_file(temp_dir.path().join("src/models/models.rs"), models_rs)?;
    write_file(temp_dir.path().join("src/utils/utils.rs"), utils_rs)?;
    write_file(temp_dir.path().join("tests/test_utils.rs"), test_rs)?;
    write_file(temp_dir.path().join("src/utils/helpers.js"), js_file)?;

    Ok(temp_dir)
}

fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[test]
fn test_glob_search_single_pattern() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for all Rust files
    let pattern = format!("{}/**/*.rs", temp_dir.path().display());
    let matches = SearchTools::glob_search(&pattern)?;

    assert_eq!(matches.len(), 4, "Should find 4 .rs files");

    // Check that all expected files are found
    let file_names: Vec<String> = matches
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    assert!(file_names.contains(&"main.rs".to_string()));
    assert!(file_names.contains(&"models.rs".to_string()));
    assert!(file_names.contains(&"utils.rs".to_string()));
    assert!(file_names.contains(&"test_utils.rs".to_string()));

    Ok(())
}

#[test]
fn test_glob_search_in_dir() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for Rust files in src directory
    let dir_path = temp_dir.path().join("src");
    let matches = SearchTools::glob_search_in_dir(&dir_path, "**/*.rs")?;

    assert_eq!(matches.len(), 3, "Should find 3 .rs files in src directory");

    // Search for Rust files in tests directory
    let test_dir_path = temp_dir.path().join("tests");
    let test_matches = SearchTools::glob_search_in_dir(&test_dir_path, "**/*.rs")?;

    assert_eq!(
        test_matches.len(),
        1,
        "Should find 1 .rs file in tests directory"
    );

    Ok(())
}

#[test]
fn test_glob_search_with_multiple_patterns() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for all JavaScript files
    let js_pattern = format!("{}/**/*.js", temp_dir.path().display());
    let js_matches = SearchTools::glob_search(&js_pattern)?;

    assert_eq!(js_matches.len(), 1, "Should find 1 .js file");

    // Check file name
    let js_file = js_matches.first().unwrap();
    assert_eq!(js_file.file_name().unwrap().to_string_lossy(), "helpers.js");

    Ok(())
}

#[test]
fn test_glob_search_no_matches() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for non-existent file type
    let pattern = format!("{}/**/*.py", temp_dir.path().display());
    let matches = SearchTools::glob_search(&pattern)?;

    assert!(matches.is_empty(), "Should not find any .py files");

    Ok(())
}

#[test]
fn test_glob_search_specific_file() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for a specific file
    let pattern = format!("{}/src/main.rs", temp_dir.path().display());
    let matches = SearchTools::glob_search(&pattern)?;

    assert_eq!(matches.len(), 1, "Should find exactly one file");
    assert_eq!(matches[0].file_name().unwrap().to_string_lossy(), "main.rs");

    Ok(())
}

#[test]
fn test_glob_search_sorting() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Modify a file to ensure it has a newer timestamp
    std::thread::sleep(std::time::Duration::from_millis(100));
    let file_to_modify = temp_dir.path().join("src/main.rs");
    let content = fs::read_to_string(&file_to_modify)?;
    fs::write(&file_to_modify, content + "\n// Modified")?;

    // Search for all Rust files
    let pattern = format!("{}/**/*.rs", temp_dir.path().display());
    let matches = SearchTools::glob_search(&pattern)?;

    // The first file should be the one we just modified
    assert_eq!(
        matches[0].file_name().unwrap().to_string_lossy(),
        "main.rs",
        "Modified file should be first in results due to sorting by modification time"
    );

    Ok(())
}

#[test]
fn test_grep_search_simple_pattern() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for "validate" in all files
    let results = SearchTools::grep_search("validate", None, Some(temp_dir.path()))?;

    // Should find at least 3 occurrences (in models.rs, utils.rs, and test.rs)
    assert!(
        results.len() >= 3,
        "Should find at least 3 occurrences of 'validate'"
    );

    Ok(())
}

#[test]
fn test_grep_search_with_include_pattern() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for "validate" in only .rs files
    let results = SearchTools::grep_search("validate", Some("*.rs"), Some(temp_dir.path()))?;

    // Check if all found files are .rs files
    for (path, _, _) in &results {
        assert_eq!(
            path.extension().unwrap().to_string_lossy(),
            "rs",
            "Should only match .rs files"
        );
    }

    // Search for "validate" in only .js files
    let js_results = SearchTools::grep_search("validate", Some("*.js"), Some(temp_dir.path()))?;

    // Check if all found files are .js files
    for (path, _, _) in &js_results {
        assert_eq!(
            path.extension().unwrap().to_string_lossy(),
            "js",
            "Should only match .js files"
        );
    }

    Ok(())
}

#[test]
fn test_grep_search_complex_regex() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for function definitions with regex
    let results = SearchTools::grep_search(r"fn\s+\w+\(", None, Some(temp_dir.path()))?;

    // Should find multiple function definitions
    assert!(
        results.len() >= 5,
        "Should find at least 5 function definitions"
    );

    // Search for struct definitions
    let struct_results = SearchTools::grep_search(r"struct\s+\w+", None, Some(temp_dir.path()))?;

    // Should find at least 3 struct definitions (Config, User, Post)
    assert!(
        struct_results.len() >= 3,
        "Should find at least 3 struct definitions"
    );

    Ok(())
}

#[test]
fn test_grep_search_case_sensitivity() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for "struct" (lowercase)
    let lowercase_results = SearchTools::grep_search("struct", None, Some(temp_dir.path()))?;

    // Search for "STRUCT" (uppercase)
    let uppercase_results = SearchTools::grep_search("STRUCT", None, Some(temp_dir.path()))?;

    // Default regex search should be case-sensitive
    assert!(
        !lowercase_results.is_empty(),
        "Should find lowercase 'struct'"
    );
    assert_eq!(
        uppercase_results.len(),
        0,
        "Should not find uppercase 'STRUCT'"
    );

    // Case-insensitive search with regex flag
    let case_insensitive = SearchTools::grep_search("(?i)struct", None, Some(temp_dir.path()))?;
    assert!(
        !case_insensitive.is_empty(),
        "Case-insensitive search should find 'struct'"
    );

    Ok(())
}

#[test]
fn test_grep_search_no_matches() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Search for non-existent text
    let results = SearchTools::grep_search("xyzabc123notfound", None, Some(temp_dir.path()))?;

    assert!(results.is_empty(), "Should not find any matches");

    Ok(())
}

#[test]
fn test_grep_search_match_ordering() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Modify a file to ensure it has a newer timestamp
    std::thread::sleep(std::time::Duration::from_millis(100));
    let file_to_modify = temp_dir.path().join("src/main.rs");
    let content = fs::read_to_string(&file_to_modify)?;
    fs::write(
        &file_to_modify,
        content + "\n// Modified with validate function",
    )?;

    // Search for "validate"
    let results = SearchTools::grep_search("validate", None, Some(temp_dir.path()))?;

    // First result should be from the modified file
    if !results.is_empty() {
        assert!(
            results[0].0.to_string_lossy().contains("main.rs"),
            "First result should be from the most recently modified file"
        );
    }

    Ok(())
}

#[test]
fn test_glob_to_regex_conversion() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // Test the include pattern with braces syntax
    let results = SearchTools::grep_search("validate", Some("*.{rs,js}"), Some(temp_dir.path()))?;

    // Check if all found files are either .rs or .js files
    for (path, _, _) in &results {
        let ext = path.extension().unwrap().to_string_lossy();
        assert!(
            ext == "rs" || ext == "js",
            "Should only match .rs or .js files, got: {}",
            ext
        );
    }

    Ok(())
}

#[test]
fn test_combined_glob_and_grep() -> Result<()> {
    let temp_dir = setup_test_directory()?;

    // First find all Rust files
    let pattern = format!("{}/**/*.rs", temp_dir.path().display());
    let rs_files = SearchTools::glob_search(&pattern)?;

    // Then grep within those files
    let mut grep_results = Vec::new();
    for file in &rs_files {
        if let Ok(results) = SearchTools::grep_search("fn", None, Some(file.parent().unwrap())) {
            for result in results {
                if result.0 == *file {
                    grep_results.push(result);
                }
            }
        }
    }

    // Should find multiple function definitions
    assert!(
        !grep_results.is_empty(),
        "Should find function definitions in Rust files"
    );

    Ok(())
}
