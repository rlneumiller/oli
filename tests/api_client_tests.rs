//! Unit tests for the API client module

use oli_tui::apis::api_client::{
    CompletionOptions, Message, SessionManager, ToolCall, ToolDefinition, ToolResult,
};
use serde_json::json;

/// Tests for the Message struct
#[test]
fn test_message_creation() {
    // Test system message creation
    let system_msg = Message::system("Test system message".to_string());
    assert_eq!(system_msg.role, "system");
    assert_eq!(system_msg.content, "Test system message");

    // Test user message creation
    let user_msg = Message::user("Test user message".to_string());
    assert_eq!(user_msg.role, "user");
    assert_eq!(user_msg.content, "Test user message");

    // Test assistant message creation
    let assistant_msg = Message::assistant("Test assistant message".to_string());
    assert_eq!(assistant_msg.role, "assistant");
    assert_eq!(assistant_msg.content, "Test assistant message");
}

/// Tests for the SessionManager
#[cfg(test)]
mod session_manager_tests {
    use super::*;

    #[test]
    fn test_session_manager_creation() {
        // Test default creation
        let default_manager = SessionManager::default();
        assert_eq!(default_manager.messages.len(), 0);
        assert_eq!(default_manager.max_messages, 100);
        assert!(default_manager.system_message.is_none());

        // Test custom max_messages
        let custom_manager = SessionManager::new(50);
        assert_eq!(custom_manager.max_messages, 50);
    }

    #[test]
    fn test_with_system_message() {
        let manager =
            SessionManager::default().with_system_message("Test system message".to_string());
        assert!(manager.system_message.is_some());
        let system_msg = manager.system_message.unwrap();
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content, "Test system message");
    }

    #[test]
    fn test_add_messages() {
        let mut manager = SessionManager::default();

        // Add user message
        manager.add_user_message("Test user message".to_string());
        assert_eq!(manager.messages.len(), 1);
        assert_eq!(manager.messages[0].role, "user");

        // Add assistant message
        manager.add_assistant_message("Test assistant message".to_string());
        assert_eq!(manager.messages.len(), 2);
        assert_eq!(manager.messages[1].role, "assistant");

        // Test message_count
        assert_eq!(manager.message_count(), 2);
    }

    #[test]
    fn test_get_messages_for_api() {
        // Without system message
        let mut manager = SessionManager::default();
        manager.add_user_message("Test user message".to_string());
        let api_messages = manager.get_messages_for_api();
        assert_eq!(api_messages.len(), 1);

        // With system message
        let mut manager =
            SessionManager::default().with_system_message("Test system message".to_string());
        manager.add_user_message("Test user message".to_string());
        let api_messages = manager.get_messages_for_api();
        assert_eq!(api_messages.len(), 2);
        assert_eq!(api_messages[0].role, "system");
    }

    #[test]
    fn test_clear() {
        let mut manager = SessionManager::default();
        manager.add_user_message("Test user message".to_string());
        manager.add_assistant_message("Test assistant message".to_string());
        assert_eq!(manager.message_count(), 2);

        manager.clear();
        assert_eq!(manager.message_count(), 0);
    }

    #[test]
    fn test_replace_with_summary() {
        let mut manager = SessionManager::default();
        manager.add_user_message("Message 1".to_string());
        manager.add_assistant_message("Message 2".to_string());
        manager.add_user_message("Message 3".to_string());
        assert_eq!(manager.message_count(), 3);

        manager.replace_with_summary("This is a summary".to_string());
        assert_eq!(manager.message_count(), 1);
        assert_eq!(manager.messages[0].role, "system");
        assert!(manager.messages[0].content.contains("This is a summary"));
    }

    #[test]
    fn test_trim_if_needed() {
        let mut manager = SessionManager::new(3); // Max 3 messages

        // Add 5 messages to trigger trimming
        manager.add_user_message("Message 1".to_string());
        manager.add_assistant_message("Message 2".to_string());
        manager.add_user_message("Message 3".to_string());
        manager.add_assistant_message("Message 4".to_string());
        manager.add_user_message("Message 5".to_string());

        // Should trim to keep only the 3 most recent messages
        assert_eq!(manager.message_count(), 3);
        assert_eq!(manager.messages[0].content, "Message 3");
        assert_eq!(manager.messages[1].content, "Message 4");
        assert_eq!(manager.messages[2].content, "Message 5");
    }
}

/// Tests for CompletionOptions
#[test]
fn test_completion_options() {
    // Test default options
    let default_options = CompletionOptions::default();
    assert_eq!(default_options.temperature, Some(0.7));
    assert_eq!(default_options.top_p, Some(0.9));
    assert_eq!(default_options.max_tokens, Some(2048));
    assert!(default_options.tools.is_none());
    assert!(default_options.json_schema.is_none());
    assert!(!default_options.require_tool_use);

    // Test custom options
    let tools = vec![ToolDefinition {
        name: "test_tool".to_string(),
        description: "Test tool description".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string"
                }
            }
        }),
    }];

    let custom_options = CompletionOptions {
        temperature: Some(0.5),
        top_p: Some(0.8),
        max_tokens: Some(1024),
        tools: Some(tools),
        json_schema: Some("{\"type\":\"object\"}".to_string()),
        require_tool_use: true,
    };

    assert_eq!(custom_options.temperature, Some(0.5));
    assert_eq!(custom_options.top_p, Some(0.8));
    assert_eq!(custom_options.max_tokens, Some(1024));
    assert!(custom_options.tools.is_some());
    assert_eq!(custom_options.tools.as_ref().unwrap()[0].name, "test_tool");
    assert!(custom_options.json_schema.is_some());
    assert!(custom_options.require_tool_use);
}

/// Tests for ToolDefinition and ToolCall
#[test]
fn test_tool_structures() {
    // Test ToolDefinition
    let tool_def = ToolDefinition {
        name: "test_tool".to_string(),
        description: "Test tool description".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string"
                }
            }
        }),
    };

    assert_eq!(tool_def.name, "test_tool");
    assert_eq!(tool_def.description, "Test tool description");
    assert!(tool_def.parameters.is_object());

    // Test ToolCall
    let tool_call = ToolCall {
        id: Some("call_123".to_string()),
        name: "test_tool".to_string(),
        arguments: json!({
            "param1": "test_value"
        }),
    };

    assert_eq!(tool_call.id, Some("call_123".to_string()));
    assert_eq!(tool_call.name, "test_tool");
    assert!(tool_call.arguments.is_object());
    assert_eq!(tool_call.arguments["param1"], "test_value");

    // Test ToolResult
    let tool_result = ToolResult {
        tool_call_id: "call_123".to_string(),
        output: "test_output".to_string(),
    };

    assert_eq!(tool_result.tool_call_id, "call_123");
    assert_eq!(tool_result.output, "test_output");
}
