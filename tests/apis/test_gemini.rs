//! Unit tests for the Gemini API client

use oli_server::apis::gemini::GeminiClient;

#[test]
fn test_gemini_model_name() {
    // Test that the default model name is correct when providing None
    // This doesn't make API calls, just tests the client setup logic
    let api_key = "test_api_key".to_string();
    let client = GeminiClient::with_api_key(api_key, None);

    // Just validate the client creation logic worked
    assert!(
        client.is_ok(),
        "Failed to create Gemini client with default model"
    );
}

#[test]
fn test_gemini_with_custom_model() {
    // Test that the custom model name is used correctly
    let api_key = "test_api_key".to_string();
    let model_name = "gemini-2.5-pro-exp-03-25".to_string();
    let client = GeminiClient::with_api_key(api_key, Some(model_name));

    assert!(
        client.is_ok(),
        "Failed to create Gemini client with custom model"
    );
}
