//! Unit tests for the API client enum and traits

use std::sync::Arc;

// Removed unused imports
use oli_tui::apis::anthropic::AnthropicClient;
use oli_tui::apis::api_client::ApiClientEnum;
use oli_tui::apis::openai::OpenAIClient;

// MockApiClient implementation removed to eliminate dead code warning

/// Helper function to create a mock ApiClientEnum for testing
fn create_mock_client_enum() -> ApiClientEnum {
    // This is just for testing the enum dispatch mechanism,
    // we're not testing the actual API clients
    let default_model = Some("test-model".to_string());

    // Set environment variables for keys (only for test context)
    std::env::set_var("ANTHROPIC_API_KEY", "test_key");
    let client = AnthropicClient::with_api_key("test_key".to_string(), default_model).unwrap();

    ApiClientEnum::Anthropic(Arc::new(client))
}

/// Test helper to create a mock OpenAI client enum
fn create_mock_openai_enum() -> ApiClientEnum {
    // Set environment variables for keys (only for test context)
    std::env::set_var("OPENAI_API_KEY", "test_key");
    let client =
        OpenAIClient::with_api_key("test_key".to_string(), Some("test-model".to_string())).unwrap();

    ApiClientEnum::OpenAi(Arc::new(client))
}

/// Tests that ensure ApiClientEnum properly delegates to the underlying client
#[tokio::test]
async fn test_api_client_enum_construction() {
    // This test just makes sure we can create the enums without panicking
    let _anthropic_enum = create_mock_client_enum();
    let _openai_enum = create_mock_openai_enum();
}

/// Test enum pattern matching behavior
#[test]
fn test_enum_variant_matching() {
    let anthropic_enum = create_mock_client_enum();
    let openai_enum = create_mock_openai_enum();

    match anthropic_enum {
        ApiClientEnum::Anthropic(_) => {
            // Expected case
        }
        _ => {
            // Should not reach here
            unreachable!("Expected Anthropic variant");
        }
    }

    match openai_enum {
        ApiClientEnum::OpenAi(_) => {
            // Expected case
        }
        _ => {
            // Should not reach here
            unreachable!("Expected OpenAI variant");
        }
    }
}
