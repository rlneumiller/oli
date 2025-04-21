//! Unit tests for the Anthropic API client

use oli_server::apis::anthropic::{AnthropicClient, AnthropicContent, SystemContent};
use oli_server::apis::api_client::{Message, ToolDefinition};
use serde_json::json;

#[test]
fn test_anthropic_model_name() {
    // Test that the default model name is correct when providing None
    // This doesn't make API calls, just tests the client setup logic
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Verify the model name is the expected default
    assert_eq!(
        client.get_model_name(),
        "claude-3-7-sonnet-20250219",
        "Default model name should be claude-3-7-sonnet-20250219"
    );
}

#[test]
fn test_anthropic_with_custom_model() {
    // Test that the custom model name is used correctly
    let api_key = "test_api_key".to_string();
    let model_name = "claude-3-opus-20240229".to_string();
    let client = AnthropicClient::with_api_key(api_key, Some(model_name.clone())).unwrap();

    // Verify the custom model name is used
    assert_eq!(
        client.get_model_name(),
        model_name,
        "Custom model name should be used"
    );
}

#[test]
fn test_ephemeral_cache_creation() {
    // Test the helper method for creating cache control
    let cache = AnthropicClient::create_ephemeral_cache();

    assert_eq!(
        cache.cache_type, "ephemeral",
        "Cache type should be ephemeral"
    );
}

#[test]
fn test_system_message_extraction() {
    // Create a test client
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Create test messages including a system message
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        },
    ];

    // Extract the system message
    let system_content = client.extract_system_message(&messages);

    // Verify the system message was correctly extracted and formatted
    assert!(
        system_content.is_some(),
        "System message should be extracted"
    );

    if let Some(SystemContent::Array(blocks)) = system_content {
        assert_eq!(blocks.len(), 1, "Should contain exactly one system block");

        let block = &blocks[0];
        assert_eq!(block.block_type, "text", "Block type should be 'text'");
        assert_eq!(
            block.text, "You are a helpful assistant.",
            "Text content should match"
        );
        assert!(
            block.cache_control.is_some(),
            "Cache control should be present"
        );

        if let Some(cache) = &block.cache_control {
            assert_eq!(
                cache.cache_type, "ephemeral",
                "Cache type should be ephemeral"
            );
        }
    } else {
        panic!("System content should be an Array variant");
    }

    // Test with no system message
    let messages_without_system = vec![Message {
        role: "user".to_string(),
        content: "Hello".to_string(),
    }];

    let system_content = client.extract_system_message(&messages_without_system);
    assert!(
        system_content.is_none(),
        "No system message should be extracted"
    );
}

#[test]
fn test_message_conversion_with_cache_control() {
    // Create a test client
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Create test messages
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        },
        Message {
            role: "assistant".to_string(),
            content: "Hi there! How can I help you today?".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "Tell me about prompt caching".to_string(),
        },
    ];

    // Convert the messages
    let anthropic_messages = client.convert_messages(messages);

    // Verify messages are converted correctly
    assert_eq!(
        anthropic_messages.len(),
        3,
        "Should have 3 messages (system filtered out)"
    );

    // First user message should have cache control (second-to-last user)
    let first_user_msg = &anthropic_messages[0];
    assert_eq!(
        first_user_msg.role, "user",
        "First message should be a user message"
    );
    assert_eq!(
        first_user_msg.content.len(),
        1,
        "Should have one content block"
    );

    if let AnthropicContent::Text {
        text,
        cache_control,
    } = &first_user_msg.content[0]
    {
        assert_eq!(text, "Hello", "Text content should match");
        assert!(
            cache_control.is_some(),
            "First user message should have cache control"
        );
    } else {
        panic!("Content should be Text variant");
    }

    // Assistant message should not have cache control
    let assistant_msg = &anthropic_messages[1];
    assert_eq!(
        assistant_msg.role, "assistant",
        "Second message should be an assistant message"
    );

    if let AnthropicContent::Text {
        text,
        cache_control,
    } = &assistant_msg.content[0]
    {
        assert_eq!(
            text, "Hi there! How can I help you today?",
            "Text content should match"
        );
        assert!(
            cache_control.is_none(),
            "Assistant message should not have cache control"
        );
    } else {
        panic!("Content should be Text variant");
    }

    // Last user message should have cache control
    let last_user_msg = &anthropic_messages[2];
    assert_eq!(
        last_user_msg.role, "user",
        "Last message should be a user message"
    );

    if let AnthropicContent::Text {
        text,
        cache_control,
    } = &last_user_msg.content[0]
    {
        assert_eq!(
            text, "Tell me about prompt caching",
            "Text content should match"
        );
        assert!(
            cache_control.is_some(),
            "Last user message should have cache control"
        );
    } else {
        panic!("Content should be Text variant");
    }
}

#[test]
fn test_message_conversion_edge_cases() {
    // Create a test client
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Test with empty messages
    let empty_messages: Vec<Message> = vec![];
    let anthropic_messages = client.convert_messages(empty_messages);
    assert!(anthropic_messages.is_empty(), "Should produce no messages");

    // Test with only a system message (which will be filtered out)
    let only_system_message = vec![Message {
        role: "system".to_string(),
        content: "You are a helpful assistant.".to_string(),
    }];

    let anthropic_messages = client.convert_messages(only_system_message);
    assert!(anthropic_messages.is_empty(), "Should produce no messages");

    // Test with a single user message
    let single_user_message = vec![Message {
        role: "user".to_string(),
        content: "Hello".to_string(),
    }];

    let anthropic_messages = client.convert_messages(single_user_message);
    assert_eq!(anthropic_messages.len(), 1, "Should produce 1 message");

    // The single user message should have cache control as it's the last user message
    if let AnthropicContent::Text { cache_control, .. } = &anthropic_messages[0].content[0] {
        assert!(
            cache_control.is_some(),
            "Single user message should have cache control"
        );
    } else {
        panic!("Content should be Text variant");
    }
}

#[test]
fn test_tool_definitions_conversion() {
    // Create a test client
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Create test tools
    let tools = vec![
        ToolDefinition {
            name: "calculator".to_string(),
            description: "Calculate mathematical expressions".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "The mathematical expression to evaluate"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "weather".to_string(),
            description: "Get weather information".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The location to get weather for"
                    }
                }
            }),
        },
    ];

    // Convert the tools
    let anthropic_tools = client.convert_tool_definitions(tools);

    // Verify tools are converted correctly
    assert_eq!(anthropic_tools.len(), 2, "Should have 2 tools");

    // First tool should not have cache control
    let first_tool = &anthropic_tools[0];
    assert_eq!(
        first_tool.name, "calculator",
        "First tool should be the calculator"
    );
    assert_eq!(
        first_tool.description.as_ref().unwrap(),
        "Calculate mathematical expressions",
        "Description should match"
    );
    assert!(
        first_tool.cache_control.is_none(),
        "First tool should not have cache control"
    );

    // Schema should have required properties
    let schema = &first_tool.schema;
    assert!(
        schema.get("$schema").is_some(),
        "Schema should have $schema property"
    );
    assert!(
        schema.get("type").is_some(),
        "Schema should have type property"
    );
    assert!(
        schema.get("properties").is_some(),
        "Schema should have properties property"
    );

    // Last tool should have cache control
    let last_tool = &anthropic_tools[1];
    assert_eq!(
        last_tool.name, "weather",
        "Last tool should be the weather tool"
    );
    assert!(
        last_tool.cache_control.is_some(),
        "Last tool should have cache control"
    );

    if let Some(cache) = &last_tool.cache_control {
        assert_eq!(
            cache.cache_type, "ephemeral",
            "Cache type should be ephemeral"
        );
    }
}

#[test]
fn test_tool_definitions_edge_cases() {
    // Create a test client
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Test with empty tools
    let empty_tools: Vec<ToolDefinition> = vec![];
    let anthropic_tools = client.convert_tool_definitions(empty_tools);
    assert!(anthropic_tools.is_empty(), "Should produce no tools");

    // Test with a single tool
    let single_tool = vec![ToolDefinition {
        name: "calculator".to_string(),
        description: "Calculate mathematical expressions".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "The mathematical expression to evaluate"
                }
            }
        }),
    }];

    let anthropic_tools = client.convert_tool_definitions(single_tool);
    assert_eq!(anthropic_tools.len(), 1, "Should produce 1 tool");

    // The single tool should have cache control as it's the last tool
    assert!(
        anthropic_tools[0].cache_control.is_some(),
        "Single tool should have cache control"
    );

    // Test with tool that has no parameters properties
    let tool_without_properties = vec![ToolDefinition {
        name: "simple".to_string(),
        description: "Simple tool".to_string(),
        parameters: json!({
            "type": "object"
        }),
    }];

    let anthropic_tools = client.convert_tool_definitions(tool_without_properties);
    assert_eq!(anthropic_tools.len(), 1, "Should produce 1 tool");

    // Schema should still be valid
    let schema = &anthropic_tools[0].schema;
    assert!(
        schema.get("$schema").is_some(),
        "Schema should have $schema property"
    );
    assert!(
        schema.get("type").is_some(),
        "Schema should have type property"
    );
    assert!(
        schema.get("properties").is_none(),
        "Schema should not have properties property"
    );
}

#[test]
fn test_caching_integration() {
    // This test simulates the complete flow to ensure all caching components work together
    let api_key = "test_api_key".to_string();
    let client = AnthropicClient::with_api_key(api_key, None).unwrap();

    // Create test messages and tools
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        },
        Message {
            role: "assistant".to_string(),
            content: "Hi there! How can I help you today?".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "Tell me about prompt caching".to_string(),
        },
    ];

    let tools = vec![ToolDefinition {
        name: "calculator".to_string(),
        description: "Calculate mathematical expressions".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "The mathematical expression to evaluate"
                }
            }
        }),
    }];

    // Extract system message
    let system_content = client.extract_system_message(&messages);
    assert!(
        system_content.is_some(),
        "System message should be extracted"
    );

    // Convert messages
    let anthropic_messages = client.convert_messages(messages.clone());
    assert_eq!(anthropic_messages.len(), 3, "Should have 3 messages");

    // Convert tools
    let anthropic_tools = client.convert_tool_definitions(tools);
    assert_eq!(anthropic_tools.len(), 1, "Should have 1 tool");

    // Verify cache control is added at each stage
    // System message
    if let Some(SystemContent::Array(blocks)) = &system_content {
        assert!(
            blocks[0].cache_control.is_some(),
            "System should have cache control"
        );
    }

    // Messages: first and last user message should have cache control
    let user_messages_with_cache = anthropic_messages
        .iter()
        .filter(|msg| msg.role == "user")
        .filter(|msg| {
            if let AnthropicContent::Text { cache_control, .. } = &msg.content[0] {
                cache_control.is_some()
            } else {
                false
            }
        })
        .count();

    assert_eq!(
        user_messages_with_cache, 2,
        "Two user messages should have cache control"
    );

    // Tool: the single tool should have cache control
    assert!(
        anthropic_tools[0].cache_control.is_some(),
        "Tool should have cache control"
    );
}
