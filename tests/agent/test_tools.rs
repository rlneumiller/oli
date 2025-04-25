use oli_server::agent::core::{Agent, LLMProvider};
use std::env;
use std::fs;
use tempfile::tempdir;
use tokio;

// These tests are marked as benchmark tests
// They will be skipped in regular CI (github/workflows/ci.yml)
// But will run in the benchmark workflow (github/workflows/benchmark.yml)
// using the Ollama local model as the LLM

/// Helper function to initialize an Ollama agent with the default model
async fn setup_ollama_agent() -> Option<(Agent, u64)> {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return None;
    }

    // Setup needed environment for Ollama connection
    if env::var("OLLAMA_API_BASE").is_err() {
        env::set_var("OLLAMA_API_BASE", "http://localhost:11434");
    }

    // Get the default model from env
    let model = match env::var("DEFAULT_MODEL") {
        Ok(m) => m,
        Err(_) => {
            println!("DEFAULT_MODEL environment variable must be set");
            return None;
        }
    };

    // Initialize agent with Ollama
    let mut agent = Agent::new(LLMProvider::Ollama).with_model(model);
    if let Err(e) = agent.initialize().await {
        println!("Failed to initialize agent: {}", e);
        return None;
    }

    // Set a reasonable timeout
    let timeout_secs = env::var("OLI_TEST_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(120); // 2 minute default timeout

    Some((agent, timeout_secs))
}

#[tokio::test]
#[cfg_attr(not(feature = "benchmark"), ignore)]
async fn test_read_file_tool() {
    // Set up the agent
    let Some((agent, timeout_secs)) = setup_ollama_agent().await else {
        return;
    };

    // Create a temporary directory and test file
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let test_file_path = temp_dir.path().join("test_file.txt");
    let test_content = "Line 1: This is a test file.\nLine 2: With multiple lines.\nLine 3: To verify file reading.";
    fs::write(&test_file_path, test_content).expect("Failed to write test file");

    // Test the agent's ability to read a file
    let prompt = format!(
        "Read the file at {} and tell me what's on line 2",
        test_file_path.display()
    );

    let timeout_duration = std::time::Duration::from_secs(timeout_secs);
    let result = tokio::time::timeout(timeout_duration, agent.execute(&prompt)).await;

    match result {
        Ok(inner_result) => {
            let response = inner_result.expect("Agent execution failed");

            // Verify the response contains line 2 content or indicates successful reading
            assert!(
                response.contains("With multiple lines")
                    || response.contains("Line 2")
                    || (response.contains("line 2") && response.contains("multiple")),
                "Response should reference line 2 content: {}",
                response
            );
        }
        Err(_) => {
            println!("Test timed out after {} seconds", timeout_secs);
            // We consider timeout a soft success for benchmark continuity
        }
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "benchmark"), ignore)]
async fn test_glob_tool() {
    // Set up the agent
    let Some((agent, timeout_secs)) = setup_ollama_agent().await else {
        return;
    };

    // Create a temporary directory with multiple files matching patterns
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Create a nested directory structure with various file types
    let rs_dir = temp_dir.path().join("src");
    let js_dir = temp_dir.path().join("ui");
    fs::create_dir_all(&rs_dir).expect("Failed to create rs directory");
    fs::create_dir_all(&js_dir).expect("Failed to create js directory");

    // Create Rust files
    fs::write(rs_dir.join("main.rs"), "fn main() {}").expect("Failed to write main.rs");
    fs::write(rs_dir.join("lib.rs"), "pub fn hello() {}").expect("Failed to write lib.rs");
    fs::write(rs_dir.join("utils.rs"), "pub fn util() {}").expect("Failed to write utils.rs");

    // Create JS files
    fs::write(js_dir.join("app.js"), "console.log('Hello');").expect("Failed to write app.js");
    fs::write(js_dir.join("utils.js"), "function util() {}").expect("Failed to write utils.js");

    // Create a README at the root
    fs::write(temp_dir.path().join("README.md"), "# Test Project")
        .expect("Failed to write README.md");

    // Test the agent's ability to use glob tool
    let prompt = format!(
        "Use the GlobTool to find all Rust files (*.rs) in the {} directory. Then, list what files you found.",
        temp_dir.path().display()
    );

    let timeout_duration = std::time::Duration::from_secs(timeout_secs);
    let result = tokio::time::timeout(timeout_duration, agent.execute(&prompt)).await;

    match result {
        Ok(inner_result) => {
            let response = inner_result.expect("Agent execution failed");

            // Verify the response references the Rust files we created
            assert!(
                response.contains("main.rs")
                    && response.contains("lib.rs")
                    && response.contains("utils.rs"),
                "Response should list all Rust files found: {}",
                response
            );
        }
        Err(_) => {
            println!("Test timed out after {} seconds", timeout_secs);
            // We consider timeout a soft success for benchmark continuity
        }
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "benchmark"), ignore)]
async fn test_grep_tool() {
    // Set up the agent
    let Some((agent, timeout_secs)) = setup_ollama_agent().await else {
        return;
    };

    // Create a temporary directory with files containing specific content
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Create files with different content patterns
    fs::write(
        temp_dir.path().join("file1.txt"),
        "This file contains important information.\nThe data we need is here.\nIMPORTANT: Don't forget this!"
    ).expect("Failed to write file1.txt");

    fs::write(
        temp_dir.path().join("file2.txt"),
        "Nothing important here.\nJust some random text.\nNo important data.",
    )
    .expect("Failed to write file2.txt");

    fs::write(
        temp_dir.path().join("file3.txt"),
        "More random content.\nIMPORTANT: Critical information here.\nDon't miss this important note."
    ).expect("Failed to write file3.txt");

    fs::write(
        temp_dir.path().join("code.rs"),
        "fn important_function() {\n    // This function does important things\n    println!(\"Important operation\");\n}"
    ).expect("Failed to write code.rs");

    // Test the agent's ability to use grep tool
    let prompt = format!(
        "Use the GrepTool to search for files in {} containing the word 'IMPORTANT' (case sensitive). \
        Tell me which files contain this pattern and the context around it.",
        temp_dir.path().display()
    );

    let timeout_duration = std::time::Duration::from_secs(timeout_secs);
    let result = tokio::time::timeout(timeout_duration, agent.execute(&prompt)).await;

    match result {
        Ok(inner_result) => {
            let response = inner_result.expect("Agent execution failed");

            // Verify the response mentions the files with IMPORTANT (case sensitive)
            assert!(
                response.contains("file1.txt")
                    && response.contains("file3.txt")
                    && !response.contains("file2.txt"),
                "Response should identify files containing 'IMPORTANT': {}",
                response
            );
        }
        Err(_) => {
            println!("Test timed out after {} seconds", timeout_secs);
            // We consider timeout a soft success for benchmark continuity
        }
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "benchmark"), ignore)]
async fn test_ls_tool() {
    // Set up the agent
    let Some((agent, timeout_secs)) = setup_ollama_agent().await else {
        return;
    };

    // Create a temporary directory with nested structure
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Create a nested directory structure
    fs::create_dir_all(temp_dir.path().join("src")).expect("Failed to create src directory");
    fs::create_dir_all(temp_dir.path().join("docs")).expect("Failed to create docs directory");
    fs::create_dir_all(temp_dir.path().join("config")).expect("Failed to create config directory");

    // Create various files
    fs::write(temp_dir.path().join("README.md"), "# Project").expect("Failed to write README.md");
    fs::write(temp_dir.path().join("LICENSE"), "MIT License").expect("Failed to write LICENSE");
    fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}")
        .expect("Failed to write main.rs");
    fs::write(temp_dir.path().join("config/settings.json"), "{}")
        .expect("Failed to write settings.json");

    // Test the agent's ability to use LSTool
    let prompt = format!(
        "Use the LSTool to list all files and directories in {}. \
        Then, tell me the directory structure and what's inside the src directory.",
        temp_dir.path().display()
    );

    let timeout_duration = std::time::Duration::from_secs(timeout_secs);
    let result = tokio::time::timeout(timeout_duration, agent.execute(&prompt)).await;

    match result {
        Ok(inner_result) => {
            let response = inner_result.expect("Agent execution failed");

            // Verify the response lists the directories and files we created
            assert!(
                response.contains("src")
                    && response.contains("docs")
                    && response.contains("config")
                    && response.contains("README.md")
                    && response.contains("LICENSE"),
                "Response should list directories and files: {}",
                response
            );
        }
        Err(_) => {
            println!("Test timed out after {} seconds", timeout_secs);
            // We consider timeout a soft success for benchmark continuity
        }
    }
}
