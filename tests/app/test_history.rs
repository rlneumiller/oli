use oli_server::app::core::{App, AppState};
use oli_server::app::history::{ContextCompressor, ConversationSummary};

#[test]
fn test_conversation_char_count() {
    let mut app = App::new();
    app.state = AppState::Chat;

    // Empty conversation should have 0 chars
    assert_eq!(app.conversation_char_count(), 0);

    // Add some messages
    app.messages.push("Hello".to_string());
    app.messages.push("World".to_string());

    // Should count "Hello" (5) + "World" (5) = 10
    assert_eq!(app.conversation_char_count(), 10);
}

#[test]
fn test_should_compress() {
    let mut app = App::new();
    app.state = AppState::Chat;

    // Empty conversation should not need summarization
    assert!(!app.should_compress());

    // Add enough messages to trigger summarization by count (1000 is the threshold in history.rs)
    for i in 0..1001 {
        app.messages.push(format!("Message {}", i));
    }

    // Should now need summarization
    assert!(app.should_compress());

    // Reset and try with character count
    app.messages.clear();

    // Add a single large message (1,000,000 chars is the threshold in history.rs)
    app.messages.push("A".repeat(1000001));

    // Should need summarization due to character count
    assert!(app.should_compress());
}

#[test]
fn test_summary_count() {
    let app = App::new();

    // Should start with 0 summaries
    assert_eq!(app.summary_count(), 0);
}

#[test]
fn test_clear_history() {
    let mut app = App::new();

    // Add some messages and summaries
    app.messages.push("Test message".to_string());

    // Add a fake summary
    app.conversation_summaries
        .push(ConversationSummary::new("Test summary".to_string(), 5, 100));

    // Clear history
    app.clear_history();

    // Summaries should be empty
    // Messages won't be empty because clear_history adds a notification message
    assert_eq!(app.messages.len(), 1);
    assert!(app.messages[0].contains("Chat history cleared"));
    assert!(app.conversation_summaries.is_empty());
}
