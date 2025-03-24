use crate::agent::core::Agent;
use crate::app::state::{App, AppState};
use crate::app::utils::Scrollable;
use anyhow::Result;
use std::time::Instant;

/// Message content threshold before considering summarization (in chars)
const DEFAULT_SUMMARIZATION_CHAR_THRESHOLD: usize = 1000000;
/// Message count threshold before considering summarization
const DEFAULT_SUMMARIZATION_COUNT_THRESHOLD: usize = 1000;
/// Maximum number of messages to keep unsummarized (recent history)
const DEFAULT_KEEP_RECENT_COUNT: usize = 20;

/// Represents a conversation summary
pub struct ConversationSummary {
    /// The summarized content
    pub content: String,
    /// When the summary was created
    pub created_at: Instant,
    /// Number of messages summarized
    pub messages_count: usize,
    /// Original character count that was summarized
    pub original_chars: usize,
}

impl ConversationSummary {
    pub fn new(content: String, messages_count: usize, original_chars: usize) -> Self {
        Self {
            content,
            created_at: Instant::now(),
            messages_count,
            original_chars,
        }
    }
}

/// History management trait for the application
pub trait HistoryManager {
    /// Generate a summary of the conversation history
    fn summarize_history(&mut self) -> Result<()>;

    /// Check if conversation should be summarized based on thresholds
    fn should_summarize(&self) -> bool;

    /// Get the total character count of conversation history
    fn conversation_char_count(&self) -> usize;

    /// Get summaries count
    fn summary_count(&self) -> usize;

    /// Clear all summaries and history
    fn clear_history(&mut self);
}

impl HistoryManager for App {
    fn summarize_history(&mut self) -> Result<()> {
        // Don't summarize if no messages
        if self.messages.is_empty() {
            return Ok(());
        }

        // Check if we have an agent for summarization
        let agent = match &self.agent {
            Some(agent) => agent.clone(),
            None => return Err(anyhow::anyhow!("No agent available for summarization")),
        };

        // Keep the most recent messages unsummarized
        let keep_recent = DEFAULT_KEEP_RECENT_COUNT.min(self.messages.len());
        let to_summarize = self.messages.len().saturating_sub(keep_recent);

        // If nothing to summarize, just return
        if to_summarize == 0 {
            return Ok(());
        }

        // Get the messages to summarize
        let messages_to_summarize = self.messages[0..to_summarize].join("\n");
        let messages_chars = messages_to_summarize.len();

        // Show a message that we're summarizing
        self.messages
            .push("[wait] ⚪ Summarizing conversation history...".into());

        // Generate the summary using the agent
        let summary = self.generate_summary_with_agent(&agent, &messages_to_summarize)?;

        // Create a new summary record
        let summary_record =
            ConversationSummary::new(summary.clone(), to_summarize, messages_chars);

        // Add the summary to the list
        self.conversation_summaries.push(summary_record);

        // Remove the summarized messages
        self.messages.drain(0..to_summarize);

        // Add the summary marker to the message list
        self.messages.insert(
            0,
            format!("💬 [CONVERSATION SUMMARY]\n{}\n[END SUMMARY]", summary),
        );

        // Add a notification
        self.messages.push(format!(
            "[success] ⏺ Summarized {} messages ({} chars)",
            to_summarize, messages_chars
        ));

        // Make sure to auto-scroll
        self.auto_scroll_to_bottom();

        Ok(())
    }

    fn should_summarize(&self) -> bool {
        // Don't summarize in non-chat state
        if self.state != AppState::Chat {
            return false;
        }

        // Check both message count and character count thresholds
        let message_count = self.messages.len();
        let char_count = self.conversation_char_count();

        message_count > DEFAULT_SUMMARIZATION_COUNT_THRESHOLD
            || char_count > DEFAULT_SUMMARIZATION_CHAR_THRESHOLD
    }

    fn conversation_char_count(&self) -> usize {
        self.messages.iter().map(|m| m.len()).sum()
    }

    fn summary_count(&self) -> usize {
        self.conversation_summaries.len()
    }

    fn clear_history(&mut self) {
        self.messages.clear();
        self.conversation_summaries.clear();

        // Reset both new and legacy scroll positions
        self.message_scroll.scroll_to_top();
        self.scroll_position = 0;

        // Also clear agent's conversation history if it exists
        if let Some(agent) = &mut self.agent {
            agent.clear_history();
        }
    }
}

impl App {
    /// Internal method to generate a summary with the agent
    fn generate_summary_with_agent(&mut self, agent: &Agent, content: &str) -> Result<String> {
        // Create a tokio runtime if needed
        let runtime = match &self.tokio_runtime {
            Some(rt) => rt,
            None => return Err(anyhow::anyhow!("Async runtime not available")),
        };

        // Create a cloned agent to avoid borrowing issues
        let agent_clone = agent.clone();

        // Copy the content for the async block
        let content_to_summarize = content.to_string();

        // Define the summarization prompt
        let prompt = format!(
            "You're assisting with summarizing the conversation history. Please create a CONCISE summary of the following conversation, focusing on:\n\
            - Key questions and tasks the user asked about\n\
            - Important code changes, file edits, or information discovered\n\
            - Main concepts discussed and solutions provided\n\
            \n\
            The summary should maintain coherence for future context while being as brief as possible. Focus on capturing essential context needed for continuing the conversation.\n\
            \n\
            CONVERSATION TO SUMMARIZE:\n{}", 
            content_to_summarize
        );

        // Execute the summarization
        let result = runtime.block_on(async { agent_clone.execute(&prompt).await })?;

        Ok(result)
    }
}
