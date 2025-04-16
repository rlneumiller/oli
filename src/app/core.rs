use crate::agent::core::Agent;
use crate::apis::api_client::{ApiClient, SessionManager};
use crate::app::history::ConversationSummary;
use crate::app::logger::{format_log_with_color, LogLevel};
use crate::models;
use crate::models::ModelConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::runtime::Runtime;
use uuid::Uuid;

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
        duration_secs: u64,
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
    pub created_at: u64, // Unix timestamp
    pub updated_at: u64, // Unix timestamp
    pub tool_count: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Task {
    /// Create a new in-progress task
    pub fn new(description: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

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
    pub fn complete(&mut self, output_tokens: u32) {
        // Calculate duration from task creation to now
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let duration_secs = now - self.created_at;

        // Store the output tokens
        self.output_tokens = output_tokens;

        self.status = TaskStatus::Completed {
            duration_secs,
            tool_uses: self.tool_count,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
        };
        self.updated_at = now;
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: &str) {
        self.status = TaskStatus::Failed(error.to_string());
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Increment tool count
    pub fn add_tool_use(&mut self) {
        self.tool_count += 1;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Add input tokens
    pub fn add_input_tokens(&mut self, tokens: u32) {
        self.input_tokens += tokens;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if this task is still in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status, TaskStatus::InProgress)
    }
}

/// Main backend application state
pub struct App {
    pub state: AppState,
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
    pub tasks: Vec<Task>,
    pub current_task_id: Option<String>,
    pub conversation_summaries: Vec<ConversationSummary>,
    pub session_manager: Option<SessionManager>,
    pub session_id: String,
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        // Load environment variables
        let _ = dotenv::dotenv();

        // Create tokio runtime for async operations
        let tokio_runtime = Runtime::new().ok();

        // Get current working directory
        let current_working_dir = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string());

        // Initialize the session manager
        let session_manager = Some(
            SessionManager::new(100)
                .with_system_message(crate::prompts::DEFAULT_SESSION_PROMPT.to_string()),
        );

        // Generate a unique session ID
        let session_id = Uuid::new_v4().to_string();

        Self {
            state: AppState::Setup,
            messages: vec![],
            logs: vec![],
            available_models: models::get_available_models(),
            error_message: None,
            last_query_time: std::time::Instant::now(),
            use_agent: false,
            agent: None,
            tokio_runtime,
            api_key: None,
            current_working_dir,
            tasks: Vec::new(),
            current_task_id: None,
            conversation_summaries: Vec::new(),
            session_manager,
            session_id,
        }
    }

    /// Get the current model configuration
    pub fn current_model(&self, index: usize) -> Result<&ModelConfig> {
        self.available_models
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("Invalid model index"))
    }

    /// Query the model with the given prompt
    pub fn query_model(&mut self, prompt: &str) -> Result<String> {
        // First gather all the info we need

        // Create a task for this query
        let _task_id = self.create_task(prompt);

        // Add processing message to logs
        eprintln!(
            "{}",
            format_log_with_color(LogLevel::Info, &format!("Processing query: '{}'", prompt))
        );

        // Update query time
        self.last_query_time = Instant::now();

        // Add to message history
        self.messages.push(format!("[user] {}", prompt));

        // Check for runtime
        if self.tokio_runtime.is_none() {
            return Err(anyhow::anyhow!("Async runtime not available"));
        }

        // Clone and collect all necessary data before any async calls

        // Use model_index from parameter (default to first model)
        let model_index = 0; // This should come from the frontend selection
        let model = match self.available_models.get(model_index) {
            Some(m) => m,
            None => return Err(anyhow::anyhow!("No models available")),
        };

        let model_name = model.name.clone();
        let model_file_name = model.file_name.clone();

        // Log model info
        eprintln!(
            "{}",
            format_log_with_color(LogLevel::Info, &format!("Using model: {}", model_name))
        );

        // API key
        let api_key = self.api_key.clone().unwrap_or_else(|| {
            std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .unwrap_or_default()
        });

        if api_key.is_empty() {
            return Err(anyhow::anyhow!("No API key available. Please set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable."));
        }

        // Session management
        if self.session_manager.is_none() {
            return Err(anyhow::anyhow!("Session manager not available"));
        }

        // Add user message to session
        if let Some(session) = &mut self.session_manager {
            session.add_user_message(prompt.to_string());
        }

        // Get messages from session
        let messages = match &self.session_manager {
            Some(session) => session.get_messages_for_api(),
            None => return Err(anyhow::anyhow!("Session manager not available")),
        };

        // Check model type and log warning if needed
        let model_name_lower = model_name.to_lowercase();
        let unrecognized = !model_name_lower.contains("claude")
            && !model_name_lower.contains("gpt")
            && !model_name_lower.contains("local");

        if unrecognized {
            eprintln!(
                "{}",
                format_log_with_color(
                    LogLevel::Warning,
                    &format!("Warning: Unrecognized model type: {}", model_name)
                )
            );
        }

        // Now make the API call - carefully extracting runtime to avoid borrow issues
        let runtime = self.tokio_runtime.as_ref().unwrap();
        let options = crate::apis::api_client::CompletionOptions {
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(2048),
            ..Default::default()
        };

        // Execute the appropriate API call
        let response = if model_name_lower.contains("claude") {
            // Use Anthropic API for Claude models
            runtime.block_on(async {
                let client = crate::apis::anthropic::AnthropicClient::with_api_key(
                    api_key.clone(),
                    Some(model_file_name.clone()),
                )?;

                client.complete(messages.clone(), options).await
            })?
        } else if model_name_lower.contains("gpt") {
            // Use OpenAI API for GPT models
            runtime.block_on(async {
                let client = crate::apis::openai::OpenAIClient::with_api_key(
                    api_key.clone(),
                    Some(model_file_name.clone()),
                )?;

                client.complete(messages.clone(), options).await
            })?
        } else if model_name_lower.contains("local") {
            // Use Ollama API for local models
            runtime.block_on(async {
                let client = crate::apis::ollama::OllamaClient::new(Some(model_file_name.clone()))?;

                client.complete(messages.clone(), options).await
            })?
        } else {
            // Fallback to a default message if the model is not recognized
            format!("I couldn't send your message to a language model. The model '{}' is not currently supported.", model_name)
        };

        // Add the assistant response to the session
        if let Some(session) = &mut self.session_manager {
            session.add_assistant_message(response.clone());
        }

        // Add the response to the message history
        self.messages.push(format!("[assistant] {}", response));

        // Complete the task (with an estimate of output tokens)
        let estimated_tokens = (response.len() as f64 / 4.0).ceil() as u32;
        self.complete_current_task(estimated_tokens);

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Info,
                &format!(
                    "Query completed, received approximately {} tokens",
                    estimated_tokens
                )
            )
        );

        Ok(response)
    }

    /// Check if there are any active tasks
    pub fn has_active_tasks(&self) -> bool {
        self.tasks.iter().any(|task| task.is_in_progress())
    }

    /// Get the task statuses for all tasks
    pub fn get_task_statuses(&self) -> Vec<serde_json::Value> {
        self.tasks
            .iter()
            .map(|task| {
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
                    "created_at": task.created_at,
                })
            })
            .collect()
    }

    /// Create a new task and set it as current
    pub fn create_task(&mut self, description: &str) -> String {
        let task = Task::new(description);
        let task_id = task.id.clone();
        self.tasks.push(task);
        self.current_task_id = Some(task_id.clone());
        task_id
    }

    /// Get the current task if any
    pub fn current_task(&self) -> Option<&Task> {
        if let Some(id) = &self.current_task_id {
            self.tasks.iter().find(|t| &t.id == id)
        } else {
            None
        }
    }

    /// Get the current task as mutable if any
    pub fn current_task_mut(&mut self) -> Option<&mut Task> {
        if let Some(id) = &self.current_task_id {
            let id_clone = id.clone();
            self.tasks.iter_mut().find(|t| t.id == id_clone)
        } else {
            None
        }
    }

    /// Add a tool use to the current task
    pub fn add_tool_use(&mut self) {
        if let Some(task) = self.current_task_mut() {
            task.add_tool_use();
        }
    }

    /// Add input tokens to the current task
    pub fn add_input_tokens(&mut self, tokens: u32) {
        if let Some(task) = self.current_task_mut() {
            task.add_input_tokens(tokens);
        }
    }

    /// Complete the current task
    pub fn complete_current_task(&mut self, tokens: u32) {
        if let Some(task) = self.current_task_mut() {
            task.complete(tokens);
        }
        self.current_task_id = None;
    }

    /// Mark the current task as failed
    pub fn fail_current_task(&mut self, error: &str) {
        if let Some(task) = self.current_task_mut() {
            task.fail(error);
        }
        self.current_task_id = None;
    }

    /// Add a log message (now deprecated in favor of direct eprintln calls)
    #[deprecated(
        since = "0.2.0",
        note = "Use eprintln with format_log_with_color instead"
    )]
    pub fn log(&mut self, _message: &str) {
        // This function is kept for backward compatibility but should not be used
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
