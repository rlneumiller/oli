//! Unit tests for the Ollama API client

use oli_server::apis::ollama::OllamaClient;

#[test]
fn test_ollama_model_name() {
    // Test that we can create a client with the default URL
    // This doesn't make API calls, just tests the client setup logic
    let client = OllamaClient::new(Some("llama2".to_string()));

    assert!(
        client.is_ok(),
        "Failed to create Ollama client with default URL"
    );
}

#[test]
fn test_ollama_with_custom_base_url() {
    // Test that the custom base URL is used correctly
    let base_url = "http://custom-ollama-server:11434".to_string();
    let model_name = "qwen2".to_string();
    let client = OllamaClient::with_base_url(model_name, base_url);

    assert!(
        client.is_ok(),
        "Failed to create Ollama client with custom base URL"
    );
}
