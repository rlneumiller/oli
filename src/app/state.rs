use crate::agent::core::Agent;
use crate::apis::api_client::SessionManager;
use crate::app::commands::SpecialCommand;
use crate::app::history::ConversationSummary;
use crate::app::models::ToolPermissionStatus;
use crate::app::permissions::PendingToolExecution;
use crate::app::utils::ScrollState;
use crate::models::ModelConfig;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone)]
pub enum AppState {
    Setup,
    ApiKeyInput,
    Error(String),
    Chat,
}

/// Status of a task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// Task is in progress/ongoing
    InProgress,
    /// Task completed successfully
    Completed {
        duration: Duration,
        tool_uses: u32,
        input_tokens: u32,
        output_tokens: u32,
    },
    /// Task failed
    Failed(String),
}

/// Represents a task the assistant is working on
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: Instant,
    pub updated_at: Instant,
    pub tool_count: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Task {
    /// Create a new in-progress task
    pub fn new(description: &str) -> Self {
        let now = Instant::now();
        Self {
            id: format!("{}", Uuid::new_v4().simple()),
            description: description.to_string(),
            status: TaskStatus::InProgress,
            created_at: now,
            updated_at: now,
            tool_count: 0,
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    /// Mark task as completed
    pub fn complete(&mut self, _tool_uses: u32, output_tokens: u32) {
        // Calculate duration from task creation to now, not just since last update
        let now = Instant::now();
        let duration = now.duration_since(self.created_at);

        // Store the output tokens
        self.output_tokens = output_tokens;

        self.status = TaskStatus::Completed {
            duration,
            tool_uses: self.tool_count, // Use actual tool count from task
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
        };
        self.updated_at = now;
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: &str) {
        self.status = TaskStatus::Failed(error.to_string());
        self.updated_at = Instant::now();
    }

    /// Increment tool count
    pub fn add_tool_use(&mut self) {
        self.tool_count += 1;
        self.updated_at = Instant::now();
    }

    /// Add input tokens
    pub fn add_input_tokens(&mut self, tokens: u32) {
        self.input_tokens += tokens;
        self.updated_at = Instant::now();
    }

    /// Check if this task is still in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status, TaskStatus::InProgress)
    }
}

use tui_textarea::TextArea;

// Add Clone implementation for App
impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            textarea: TextArea::default(), // Cannot clone TextArea, create default
            input: self.input.clone(),
            messages: self.messages.clone(),
            logs: self.logs.clone(),
            show_logs: self.show_logs,
            selected_model: self.selected_model,
            available_models: self.available_models.clone(),
            error_message: self.error_message.clone(),
            debug_messages: self.debug_messages,
            message_scroll: self.message_scroll.clone(),
            log_scroll: self.log_scroll.clone(),
            scroll_position: self.scroll_position,
            last_query_time: self.last_query_time,
            last_message_time: self.last_message_time,
            use_agent: self.use_agent,
            agent: None,             // Cannot clone Agent, use None
            tokio_runtime: None,     // Cannot clone Runtime, use None
            agent_progress_rx: None, // Cannot clone Receiver, use None
            api_key: self.api_key.clone(),
            current_working_dir: self.current_working_dir.clone(),
            command_mode: self.command_mode,
            available_commands: self.available_commands.clone(),
            selected_command: self.selected_command,
            show_command_menu: self.show_command_menu,
            permission_required: self.permission_required,
            pending_tool: self.pending_tool.clone(),
            tool_permission_status: self.tool_permission_status.clone(),
            tool_execution_in_progress: self.tool_execution_in_progress,
            show_intermediate_steps: self.show_intermediate_steps,
            show_shortcuts_hint: self.show_shortcuts_hint,
            show_detailed_shortcuts: self.show_detailed_shortcuts,
            parse_code_mode: self.parse_code_mode,
            cursor_position: self.cursor_position,
            tasks: self.tasks.clone(),
            current_task_id: self.current_task_id.clone(),
            task_scroll: self.task_scroll.clone(),
            task_scroll_position: self.task_scroll_position,
            conversation_summaries: self.conversation_summaries.clone(),
            session_manager: None, // Cannot clone SessionManager, use None
            session_id: self.session_id.clone(),
        }
    }
}

pub struct App {
    pub state: AppState,
    pub textarea: TextArea<'static>, // TextArea widget for improved multiline input
    pub input: String,               // Keep for backward compatibility during transition
    pub messages: Vec<String>,
    pub logs: Vec<String>, // Store logs separately from messages
    pub show_logs: bool,   // Toggle between logs and messages display
    pub selected_model: usize,
    pub available_models: Vec<ModelConfig>,
    pub error_message: Option<String>,
    pub debug_messages: bool,
    pub message_scroll: ScrollState, // Improved scrolling for messages
    pub log_scroll: ScrollState,     // Separate scrolling for logs
    pub scroll_position: usize,      // Legacy scroll position (kept for compatibility)
    pub last_query_time: Instant,
    pub last_message_time: Instant, // Timestamp for message animations
    pub use_agent: bool,
    pub agent: Option<Agent>,
    pub tokio_runtime: Option<Runtime>,
    pub agent_progress_rx: Option<mpsc::Receiver<String>>,
    pub api_key: Option<String>,
    pub current_working_dir: Option<String>,
    // Command-related fields
    pub command_mode: bool,
    pub available_commands: Vec<SpecialCommand>,
    pub selected_command: usize,
    pub show_command_menu: bool,
    // Tool permission-related fields
    pub permission_required: bool, // If true, we're waiting for user input on a tool permission
    pub pending_tool: Option<PendingToolExecution>, // The tool waiting for permission
    pub tool_permission_status: ToolPermissionStatus, // Current permission status
    pub tool_execution_in_progress: bool, // Flag to indicate active tool execution
    pub show_intermediate_steps: bool, // Show intermediate steps like tool use and file reads
    pub show_shortcuts_hint: bool, // Show the shortcut hint below input box
    pub show_detailed_shortcuts: bool, // Show all shortcuts when ? is pressed
    // State for special commands
    pub parse_code_mode: bool, // Flag to indicate we're in parse_code command mode waiting for file path
    // Cursor position in input - kept for backward compatibility
    pub cursor_position: usize, // Current cursor position in the input string
    // Task tracking
    pub tasks: Vec<Task>,
    pub current_task_id: Option<String>,
    pub task_scroll: ScrollState,    // Improved scrolling for task list
    pub task_scroll_position: usize, // Legacy scroll position (kept for compatibility)
    // Conversation history management
    pub conversation_summaries: Vec<ConversationSummary>, // History of conversation summaries
    // Session management for API conversation
    pub session_manager: Option<SessionManager>, // Manages the API conversation session
    // Session information for logging
    pub session_id: String, // Unique ID for the current session
}
