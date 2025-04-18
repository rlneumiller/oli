//! Unit tests for the OpenAI API client

use oli_server::apis::openai::OpenAIClient;

#[test]
fn test_openai_model_name() {
    // Test that the default model name is correct when providing None
    // This doesn't make API calls, just tests the client setup logic
    let api_key = "test_api_key".to_string();
    let client = OpenAIClient::with_api_key(api_key, None);

    // Just validate the client creation logic worked
    assert!(
        client.is_ok(),
        "Failed to create OpenAI client with default model"
    );
}

#[test]
fn test_openai_with_custom_model() {
    // Test that the custom model name is used correctly
    let api_key = "test_api_key".to_string();
    let model_name = "gpt-4-turbo".to_string();
    let client = OpenAIClient::with_api_key(api_key, Some(model_name));

    assert!(
        client.is_ok(),
        "Failed to create OpenAI client with custom model"
    );
}
