use crate::agent::core::Agent;
use crate::apis::api_client::SessionManager;
use crate::app::async_processor::AsyncProcessor;
use crate::app::commands::SpecialCommand;
use crate::app::history::ConversationSummary;
use crate::app::permissions::PendingToolExecution;
use crate::app::models::ToolPermissionStatus;
use crate::models::ModelConfig;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Backend application state enum
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum AppState {
    Setup,
    ApiKeyInput,
    Error(String),
    Ready,
    Chat,
}

/// Status of a task
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Main application state - stripped of UI components
pub struct App {
    pub state: AppState,
    pub input: String,
    pub messages: Vec<String>,
    pub logs: Vec<String>,
    pub available_models: Vec<ModelConfig>,
    pub error_message: Option<String>,
    pub last_query_time: Instant,
    pub use_agent: bool,
    pub agent: Option<Agent>,
    pub tokio_runtime: Option<Runtime>,
    pub api_key: Option<String>,
    pub current_working_dir: Option<String>,
    pub available_commands: Vec<SpecialCommand>,
    pub permission_required: bool,
    pub pending_tool: Option<PendingToolExecution>,
    pub tool_permission_status: ToolPermissionStatus,
    pub tool_execution_in_progress: bool,
    pub parse_code_mode: bool,
    pub tasks: Vec<Task>,
    pub current_task_id: Option<String>,
    pub conversation_summaries: Vec<ConversationSummary>,
    pub session_manager: Option<SessionManager>,
    pub session_id: String,
    pub model_processor: AsyncProcessor<String>,
    pub tool_processor: AsyncProcessor<String>,
    pub command_processor: AsyncProcessor<String>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        // Load environment variables from .env file if present
        let _ = dotenv::dotenv();

        // Create tokio runtime for async operations
        let tokio_runtime = Runtime::new().ok();

        // Get current working directory
        let current_working_dir = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string());

        // Initialize the session manager with default settings
        let session_manager = Some(SessionManager::new(100).with_system_message(crate::prompts::DEFAULT_SESSION_PROMPT.to_string()));

        // Generate a unique session ID
        let session_id = Uuid::new_v4().to_string();

        use crate::app::async_processor::AsyncProcessor;
        use crate::models::get_available_models;

        Self {
            state: AppState::Setup,
            input: String::new(),
            messages: vec![],
            logs: vec![],
            available_models: get_available_models(),
            error_message: None,
            last_query_time: std::time::Instant::now(),
            use_agent: false,
            agent: None,
            tokio_runtime,
            api_key: None,
            current_working_dir,
            available_commands: crate::app::commands::get_available_commands(),
            permission_required: false,
            pending_tool: None,
            tool_permission_status: ToolPermissionStatus::Pending,
            tool_execution_in_progress: false,
            parse_code_mode: false,
            tasks: Vec::new(),
            current_task_id: None,
            conversation_summaries: Vec::new(),
            session_manager,
            session_id,
            model_processor: AsyncProcessor::default(),
            tool_processor: AsyncProcessor::default(),
            command_processor: AsyncProcessor::default(),
        }
    }
    
    /// Check if there are any active tasks
    pub fn has_active_tasks(&self) -> bool {
        self.tasks.iter().any(|task| task.is_in_progress())
    }
    
    /// Get the task statuses for all tasks
    pub fn get_task_statuses(&self) -> Vec<serde_json::Value> {
        self.tasks.iter().map(|task| {
            let status = match &task.status {
                TaskStatus::InProgress => "in_progress",
                TaskStatus::Completed { .. } => "completed",
                TaskStatus::Failed(_) => "failed",
            };
            
            serde_json::json!({
                "id": task.id,
                "description": task.description,
                "status": status,
                "tool_count": task.tool_count,
                "input_tokens": task.input_tokens,
                "output_tokens": task.output_tokens,
                "created_at": task.created_at.elapsed().as_secs(),
            })
        }).collect()
    }
}