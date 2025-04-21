// Anthropic module unit tests have been moved directly into the module implementation
// This is a basic integration test to ensure client instantiation still works properly
#[cfg(test)]
mod test_anthropic {
    use oli_server::apis::anthropic::AnthropicClient;

    #[test]
    fn test_anthropic_client_instantiation() {
        // Test with a mock API key
        let api_key = "test_api_key".to_string();
        let client = AnthropicClient::with_api_key(api_key, None);

        assert!(
            client.is_ok(),
            "Should be able to create AnthropicClient with test API key"
        );
    }

    #[test]
    fn test_anthropic_with_custom_model() {
        // Test with a mock API key and custom model
        let api_key = "test_api_key".to_string();
        let model_name = "claude-3-opus-20240229".to_string();
        let client = AnthropicClient::with_api_key(api_key, Some(model_name));

        assert!(
            client.is_ok(),
            "Should be able to create client with custom model"
        );
    }
}
mod test_api_client;
mod test_api_client_enum;
mod test_gemini;
mod test_ollama;
mod test_openai;
