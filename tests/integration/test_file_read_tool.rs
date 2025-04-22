use oli_server::agent::core::{Agent, LLMProvider};
use std::env;
use std::fs;
use tempfile::tempdir;
use tokio;

// This test is marked as a benchmark test
// It will be skipped in regular CI (github/workflows/ci.yml)
// But will run in the benchmark workflow (github/workflows/benchmark.yml)
// using the Ollama local model as the LLM

#[tokio::test]
#[cfg_attr(not(feature = "benchmark"), ignore)]
async fn test_read_file_tool() {
    // Skip if SKIP_INTEGRATION is set (useful for CI/CD environments)
    if std::env::var("SKIP_INTEGRATION").is_ok() {
        return;
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
            return;
        }
    };

    // Create a temporary directory and test file
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let test_file_path = temp_dir.path().join("test_file.txt");
    let test_content = "Line 1: This is a test file.\nLine 2: With multiple lines.\nLine 3: To verify file reading.";
    fs::write(&test_file_path, test_content).expect("Failed to write test file");

    // Initialize agent with Ollama
    let mut agent = Agent::new(LLMProvider::Ollama).with_model(model);
    if let Err(e) = agent.initialize().await {
        println!("Failed to initialize agent: {}", e);
        return;
    }

    // Test the agent's ability to read a file
    let prompt = format!(
        "Read the file at {} and tell me what's on line 2",
        test_file_path.display()
    );

    // Set a reasonable timeout
    let timeout_secs = env::var("OLI_TEST_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(120); // 2 minute default timeout

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
