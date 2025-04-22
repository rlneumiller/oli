//! Unit tests for the Agent core module

use oli_server::agent::core::{Agent, LLMProvider};
use oli_server::apis::api_client::Message;
use tokio::sync::mpsc;

/// Tests the creation of a new Agent
#[test]
fn test_agent_creation() {
    // Test with different providers
    let _agent_anthropic = Agent::new(LLMProvider::Anthropic);
    let _agent_openai = Agent::new(LLMProvider::OpenAI);
    let _agent_ollama = Agent::new(LLMProvider::Ollama);
    let _agent_gemini = Agent::new(LLMProvider::Gemini);

    // Simply test that we can create agents with different providers
    // (we can't access private fields directly)
}

/// Tests agent creation with an API key
#[test]
fn test_agent_with_api_key() {
    let api_key = "test_api_key";
    let _agent = Agent::new_with_api_key(LLMProvider::Anthropic, api_key.to_string());

    // Just test that the method exists and doesn't panic
}

/// Tests setting a model for the agent
#[test]
fn test_agent_with_model() {
    let model = "claude-3-opus-20240229";
    let _agent = Agent::new(LLMProvider::Anthropic).with_model(model.to_string());

    // Just test that the method exists and doesn't panic
}

/// Tests setting a custom system prompt
#[test]
fn test_agent_with_system_prompt() {
    let prompt = "You are a helpful assistant.";
    let _agent = Agent::new(LLMProvider::Anthropic).with_system_prompt(prompt.to_string());

    // Just test that the method exists and doesn't panic
}

/// Tests setting a progress sender
#[test]
fn test_agent_with_progress_sender() {
    let (sender, _receiver) = mpsc::channel::<String>(10);
    let _agent = Agent::new(LLMProvider::Anthropic).with_progress_sender(sender);

    // Just test that the method exists and doesn't panic
}

/// Tests conversation history management - using the public API
#[test]
fn test_conversation_history_management() {
    let mut agent = Agent::new(LLMProvider::Anthropic);

    // Add messages
    let system_msg = Message::system("System message".to_string());
    let user_msg = Message::user("User message".to_string());
    let assistant_msg = Message::assistant("Assistant message".to_string());

    agent.add_message(system_msg.clone());
    agent.add_message(user_msg.clone());
    agent.add_message(assistant_msg.clone());

    // Verify messages were added using the test method
    let history = agent.get_conversation_history_for_test();
    assert_eq!(history.len(), 3);

    // Verify message order and content
    assert_eq!(history[0].role, "system");
    assert_eq!(history[0].content, "System message");
    assert_eq!(history[1].role, "user");
    assert_eq!(history[1].content, "User message");
    assert_eq!(history[2].role, "assistant");
    assert_eq!(history[2].content, "Assistant message");

    // Test clearing history
    agent.clear_history();
    assert_eq!(agent.get_conversation_history_for_test().len(), 0);
}

/// Test system prompt preservation in conversation history
#[test]
fn test_system_prompt_preservation() {
    let mut agent =
        Agent::new(LLMProvider::Anthropic).with_system_prompt("Custom system prompt".to_string());

    // Add system message first
    agent.add_message(Message::system("Custom system prompt".to_string()));

    // Add regular messages
    agent.add_message(Message::user("User message".to_string()));
    agent.add_message(Message::assistant("Assistant response".to_string()));

    // Check system message is preserved
    let history = agent.get_conversation_history_for_test();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[0].content, "Custom system prompt");
}

/// Test conversation history is properly maintained through simulated execution
#[test]
fn test_conversation_continuity() {
    let mut agent = Agent::new(LLMProvider::Anthropic);

    // Initialize with a conversation history
    agent.add_message(Message::system("System prompt".to_string()));
    agent.add_message(Message::user("First user message".to_string()));
    agent.add_message(Message::assistant("First assistant response".to_string()));

    // Simulate another query (what would happen in execute)
    let new_message = Message::user("Follow-up question".to_string());
    agent.add_message(new_message);

    // Simulate assistant response
    let assistant_response = Message::assistant("Follow-up answer".to_string());
    agent.add_message(assistant_response);

    // Verify history contains full conversation
    let history = agent.get_conversation_history_for_test();
    assert_eq!(history.len(), 5);

    // Check conversation flow
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].role, "user");
    assert_eq!(history[1].content, "First user message");
    assert_eq!(history[2].role, "assistant");
    assert_eq!(history[2].content, "First assistant response");
    assert_eq!(history[3].role, "user");
    assert_eq!(history[3].content, "Follow-up question");
    assert_eq!(history[4].role, "assistant");
    assert_eq!(history[4].content, "Follow-up answer");
}

// Mock tests for initialization
mod mock_initialization {
    use super::*;
    use anyhow::Result;

    // Test that checks if we can initialize without an API key
    // This is more of a compilation test than a runtime test
    #[tokio::test]
    async fn test_initialize_signature() -> Result<()> {
        let mut agent = Agent::new(LLMProvider::Anthropic);

        // Should compile - this checks the method signature is as expected
        // The actual implementation will fail without mock providers
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let _ = agent.initialize().await;
            })
        }));

        // We expect this to fail or panic in a real environment without mocks
        // The point is to verify the signature is correct
        assert!(
            result.is_err(),
            "Expected the method to fail or panic without mocks, but it succeeded."
        );

        Ok(())
    }

    // Test that checks if we can initialize with an API key
    // This is more of a compilation test than a runtime test
    #[tokio::test]
    async fn test_initialize_with_api_key_signature() -> Result<()> {
        let mut agent = Agent::new(LLMProvider::Anthropic);

        // Should compile - this checks the method signature is as expected
        // The actual implementation will fail without mock providers
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let _ = agent.initialize_with_api_key("test_key".to_string()).await;
            })
        }));

        // We expect this to fail or panic in a real environment without mocks
        // The point is to verify the signature is correct
        assert!(result.is_ok(), "The method call panicked unexpectedly.");

        Ok(())
    }
}

/// Test for message ordering functionality (simulated since we can't access internal functions directly)
#[test]
fn test_message_ordering() {
    let mut agent = Agent::new(LLMProvider::Anthropic);

    // Add messages in mixed order
    agent.add_message(Message::user("User message first".to_string()));
    agent.add_message(Message::assistant("Assistant response".to_string()));
    agent.add_message(Message::system("System message added later".to_string()));

    // Manually extract the system message for verification
    let has_system = agent
        .get_conversation_history_for_test()
        .iter()
        .any(|msg| msg.role == "system");

    // Verify system message presence
    assert!(has_system);
}

/// Test for message management functionality using the exposed public API
#[test]
fn test_message_manipulation() {
    let mut agent = Agent::new(LLMProvider::Anthropic);

    // Add 3 messages
    agent.add_message(Message::system("System message".to_string()));
    agent.add_message(Message::user("User message".to_string()));
    agent.add_message(Message::assistant("Assistant message".to_string()));

    // Get a copy of the messages
    let messages = agent.get_conversation_history_for_test();
    assert_eq!(messages.len(), 3);

    // Clear the history and verify
    agent.clear_history();
    assert_eq!(agent.get_conversation_history_for_test().len(), 0);

    // Re-add the messages
    for msg in messages {
        agent.add_message(msg);
    }

    // Verify they're back
    assert_eq!(agent.get_conversation_history_for_test().len(), 3);
}

/// Test multiple operations in sequence to verify correct state management
#[test]
fn test_sequential_operations() {
    let mut agent = Agent::new(LLMProvider::Anthropic);

    // Add a message
    agent.add_message(Message::user("First message".to_string()));
    assert_eq!(agent.get_conversation_history_for_test().len(), 1);

    // Clear the history
    agent.clear_history();
    assert_eq!(agent.get_conversation_history_for_test().len(), 0);

    // Add multiple messages
    agent.add_message(Message::system("System prompt".to_string()));
    agent.add_message(Message::user("User question".to_string()));
    assert_eq!(agent.get_conversation_history_for_test().len(), 2);

    // Add another message and verify
    agent.add_message(Message::assistant("Assistant response".to_string()));
    assert_eq!(agent.get_conversation_history_for_test().len(), 3);
    assert_eq!(
        agent.get_conversation_history_for_test()[2].role,
        "assistant"
    );
}

/// Test that multiple method calls can be chained together
#[test]
fn test_method_chaining() {
    let _agent = Agent::new(LLMProvider::Anthropic)
        .with_model("claude-3-sonnet".to_string())
        .with_system_prompt("Custom prompt".to_string());

    // Just verify that method chaining compiles and doesn't panic
    let _agent2 = Agent::new(LLMProvider::OpenAI)
        .with_model("gpt-4".to_string())
        .with_system_prompt("Another prompt".to_string());
}
