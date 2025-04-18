use crate::agent::core::Agent;
use crate::apis::api_client::{ApiClient, SessionManager};
use crate::app::history::ConversationSummary;
use crate::app::logger::{format_log_with_color, LogLevel};
use crate::models;
use crate::models::ModelConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
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

/// Tool execution status enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolExecutionStatus {
    /// Tool execution is in progress
    Running,
    /// Tool execution completed successfully
    Success,
    /// Tool execution failed
    Error,
}

/// Represents a tool execution with status updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: String,                                   // Unique ID for this tool execution
    pub task_id: String,                              // ID of the parent task
    pub name: String,                                 // Tool name (View, GlobTool, etc.)
    pub status: ToolExecutionStatus,                  // Running, Success, Error
    pub start_time: u64,                              // Start timestamp (milliseconds)
    pub end_time: Option<u64>, // End timestamp (milliseconds), None if still running
    pub message: String,       // Current status message
    pub metadata: HashMap<String, serde_json::Value>, // Additional metadata: file paths, line counts, etc.
}

impl ToolExecution {
    /// Create a new running tool execution
    pub fn new(task_id: &str, name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: format!("tool-{}-{}", name, Uuid::new_v4().simple()),
            task_id: task_id.to_string(),
            name: name.to_string(),
            status: ToolExecutionStatus::Running,
            start_time: now,
            end_time: None,
            message: format!("Starting {}", name),
            metadata: HashMap::new(),
        }
    }

    /// Mark tool execution as completed successfully
    pub fn complete(&mut self, message: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.status = ToolExecutionStatus::Success;
        self.end_time = Some(now);
        self.message = message.to_string();
    }

    /// Mark tool execution as failed
    pub fn fail(&mut self, error: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.status = ToolExecutionStatus::Error;
        self.end_time = Some(now);
        self.message = format!("Error: {}", error);
    }

    /// Update tool execution with a progress message
    pub fn update_progress(&mut self, message: &str) {
        self.message = message.to_string();
    }

    /// Add metadata to the tool execution
    pub fn add_metadata(&mut self, key: &str, value: serde_json::Value) {
        self.metadata.insert(key.to_string(), value);
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
    // Add tracking for tool executions
    pub tool_executions: HashMap<String, ToolExecution>,
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
            tool_executions: HashMap::new(),
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
        let task_id = self.create_task(prompt);

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
        let supports_agent = model.has_agent_support();

        // Log model info
        eprintln!(
            "{}",
            format_log_with_color(LogLevel::Info, &format!("Using model: {}", model_name))
        );

        // API key
        let api_key = self.api_key.clone().unwrap_or_else(|| {
            std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .or_else(|_| std::env::var("GEMINI_API_KEY"))
                .unwrap_or_default()
        });

        if api_key.is_empty() {
            return Err(anyhow::anyhow!("No API key available. Please set ANTHROPIC_API_KEY, OPENAI_API_KEY, or GEMINI_API_KEY environment variable."));
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
            && !model_name_lower.contains("local")
            && !model_name_lower.contains("gemini");

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

        // Channel for sending progress updates
        let (progress_tx, progress_rx) = std::sync::mpsc::channel();

        // Progress tracking thread for UI notifications
        let task_id_clone = task_id.clone();
        std::thread::spawn(move || {
            while let Ok(message) = progress_rx.recv() {
                // Emit progress events for the UI to pick up
                if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                    let _ = rpc_server.event_sender().send((
                        "processing_progress".to_string(),
                        serde_json::json!({
                            "task_id": task_id_clone,
                            "message": message
                        }),
                    ));
                }
            }
        });

        // Initialize agent if model supports it and use_agent is set
        if supports_agent && self.use_agent {
            // Working directory no longer needed for the simplified implementation
            let _working_dir = self.current_working_dir.clone().unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });

            // Determine provider from model name
            let has_anthropic_key =
                !api_key.is_empty() && std::env::var("ANTHROPIC_API_KEY").is_ok();
            let has_openai_key = !api_key.is_empty() && std::env::var("OPENAI_API_KEY").is_ok();
            let has_gemini_key = !api_key.is_empty() && std::env::var("GEMINI_API_KEY").is_ok();

            // Import agent provider enum
            use crate::agent::core::LLMProvider;

            // Determine the provider based on model name
            let provider = match model_name_lower.as_str() {
                name if name.contains("claude") => {
                    if has_anthropic_key {
                        Some(LLMProvider::Anthropic)
                    } else {
                        None
                    }
                }
                name if name.contains("gpt") => {
                    if has_openai_key {
                        Some(LLMProvider::OpenAI)
                    } else {
                        None
                    }
                }
                name if name.contains("gemini") => {
                    if has_gemini_key {
                        Some(LLMProvider::Gemini)
                    } else {
                        None
                    }
                }
                name if name.contains("local") => Some(LLMProvider::Ollama),
                _ => {
                    if has_anthropic_key {
                        Some(LLMProvider::Anthropic)
                    } else if has_openai_key {
                        Some(LLMProvider::OpenAI)
                    } else if has_gemini_key {
                        Some(LLMProvider::Gemini)
                    } else {
                        None
                    }
                }
            }
            .ok_or_else(|| anyhow::anyhow!("Could not determine provider for agent"))?;

            // Determine the agent model
            let agent_model = match model_name_lower.as_str() {
                name if name.contains("claude") => {
                    if has_anthropic_key {
                        Some("claude-3-7-sonnet-20250219".to_string())
                    } else {
                        None
                    }
                }
                name if name.contains("gpt") => {
                    if has_openai_key {
                        Some("gpt-4o".to_string())
                    } else {
                        None
                    }
                }
                name if name.contains("gemini") => {
                    if has_gemini_key {
                        Some("gemini-2.5-pro-exp-03-25".to_string())
                    } else {
                        None
                    }
                }
                name if name.contains("local") => Some(model_file_name.clone()),
                _ => None,
            }
            .ok_or_else(|| anyhow::anyhow!("Could not determine model for agent"))?;

            // Create and configure the agent with builder methods
            let mut agent = crate::agent::core::Agent::new(provider);
            agent = agent.with_model(agent_model);

            // Create Tokio channel for progress
            let (progress_tx_sender, mut progress_rx_receiver) =
                tokio::sync::mpsc::channel::<String>(100);

            // Add progress sender to agent
            agent = agent.with_progress_sender(progress_tx_sender);

            // Set up a task for processing progress messages
            let progress_tx_clone = progress_tx.clone();
            let task_id_clone2 = task_id.clone();

            // Spawn a thread to handle agent progress messages
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    while let Some(message) = progress_rx_receiver.recv().await {
                        // Special detection for View tool output
                        let is_view_output = message.lines().next()
                            .map(|first_line|
                                first_line.contains(" | ") &&
                                first_line.trim().chars().take(5).all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '|')
                            )
                            .unwrap_or(false);

                        if is_view_output {
                            if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                                // Count the number of lines in the output
                                let line_count = message.lines().count();

                                // Create a unique ID for this View tool execution
                                let tool_id = format!("{}.view-{}", task_id, std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis());

                                // Send tool status notification for View
                                let tool_status = serde_json::json!({
                                    "type": "updated",
                                    "execution": {
                                        "id": tool_id,
                                        "task_id": task_id,
                                        "name": "View",
                                        "status": "success",
                                        "startTime": std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis(),
                                        "endTime": std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis() + 100, // Add 100ms to ensure endTime > startTime
                                        "message": format!("Read {} lines", line_count),
                                        "metadata": {
                                            "lines": line_count,
                                            "description": format!("Read {} lines", line_count),
                                            "file_path": "view-result", // Add a placeholder file path
                                        }
                                    }
                                });

                                // Send the notification
                                rpc_server.send_notification("tool_status", tool_status).ok();
                            }
                        }

                        // Process standard tool execution events
                        if message.starts_with('[') && message.contains(']') {
                            if let Some(rpc_server) =
                                crate::communication::rpc::get_global_rpc_server()
                            {
                                let parts: Vec<&str> = message.splitn(2, ']').collect();
                                if parts.len() == 2 {
                                    let tool_name = parts[0].trim_start_matches('[').trim();
                                    let tool_message = parts[1].trim();

                                    // Log tool detection for debugging
                                    eprintln!("Detected tool message: [{}] {}", tool_name, tool_message);

                                    // Determine tool execution status - default to running
                                    let status = if message.contains("[error]")
                                        || message.contains("ERROR")
                                    {
                                        "error"
                                    } else if message.contains("[completed]")
                                        || message.contains("completed")
                                        || message.contains("success")
                                    {
                                        "success"
                                    } else {
                                        "running"
                                    };

                                    // Extract additional metadata for the tool operation
                                    let file_path = if tool_message.contains("file_path:") {
                                        let path_parts: Vec<&str> =
                                            tool_message.split("file_path:").collect();
                                        if path_parts.len() > 1 {
                                            let path_with_quotes = path_parts[1].trim();
                                            // Extract the path from quotes if present
                                            if path_with_quotes.starts_with('"')
                                                && path_with_quotes.contains('"')
                                            {
                                                let end_quote_pos = path_with_quotes[1..]
                                                    .find('"')
                                                    .map(|pos| pos + 1);
                                                end_quote_pos
                                                    .map(|pos| path_with_quotes[1..pos].to_string())
                                            } else {
                                                Some(
                                                    path_with_quotes
                                                        .split_whitespace()
                                                        .next()
                                                        .unwrap_or("")
                                                        .to_string(),
                                                )
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                    // Extract line count if available - improved detection
                                    let lines = if tool_message.contains("lines") {
                                        let line_parts: Vec<&str> =
                                            tool_message.split("lines").collect();
                                        if line_parts.len() > 1 {
                                            // Look for number right before or after "lines"
                                            let numbers: Vec<&str> = line_parts[0]
                                                .split_whitespace()
                                                .chain(line_parts[1].split_whitespace())
                                                .filter(|word| word.parse::<usize>().is_ok())
                                                .collect();

                                            numbers
                                                .first()
                                                .and_then(|num| num.parse::<usize>().ok())
                                        } else {
                                            // Fallback to original implementation
                                            tool_message
                                                .split_whitespace()
                                                .find(|word| word.parse::<usize>().is_ok())
                                                .and_then(|num| num.parse::<usize>().ok())
                                        }
                                    } else {
                                        None
                                    };

                                    // Description based on tool type
                                    let description = match tool_name {
                                        "View" => {
                                            if let Some(_path) = &file_path {
                                                if let Some(line_count) = lines {
                                                    format!(
                                                        "Read {} lines (ctrl+r to expand)",
                                                        line_count
                                                    )
                                                } else {
                                                    "Reading file contents (ctrl+r to expand)"
                                                        .to_string()
                                                }
                                            } else {
                                                "Reading file".to_string()
                                            }
                                        }
                                        "GlobTool" => "Finding files by pattern".to_string(),
                                        "GrepTool" => "Searching code for pattern".to_string(),
                                        "LS" => "Listing directory contents".to_string(),
                                        "Edit" => "Modifying file".to_string(),
                                        "Replace" => "Replacing file contents".to_string(),
                                        "Bash" => "Executing command".to_string(),
                                        _ => "Executing tool".to_string(),
                                    };

                                    // Generate a unique ID for this tool execution
                                    let tool_id = format!(
                                        "tool-{}-{}",
                                        tool_name,
                                        uuid::Uuid::new_v4().simple()
                                    );

                                    // Create timestamp
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                        as u64;

                                    // Create a ToolExecution structure
                                    let tool_execution = ToolExecution {
                                        id: tool_id.clone(),
                                        task_id: task_id_clone2.clone(),
                                        name: tool_name.to_string(),
                                        status: match status {
                                            "running" => ToolExecutionStatus::Running,
                                            "success" => ToolExecutionStatus::Success,
                                            "error" => ToolExecutionStatus::Error,
                                            _ => ToolExecutionStatus::Running,
                                        },
                                        start_time: now,
                                        end_time: if status != "running" {
                                            Some(now)
                                        } else {
                                            None
                                        },
                                        message: tool_message.to_string(),
                                        metadata: {
                                            let mut meta = std::collections::HashMap::new();
                                            if let Some(path) = &file_path {
                                                meta.insert(
                                                    "file_path".to_string(),
                                                    serde_json::Value::String(path.clone()),
                                                );
                                            }
                                            if let Some(line_count) = lines {
                                                meta.insert(
                                                    "lines".to_string(),
                                                    serde_json::Value::Number(
                                                        serde_json::Number::from(line_count),
                                                    ),
                                                );
                                            }
                                            meta.insert(
                                                "description".to_string(),
                                                serde_json::Value::String(description.clone()),
                                            );
                                            meta
                                        },
                                    };

                                    // Log the tool execution with detailed information
                                    eprintln!(
                                        "Created tool execution: {} ({}) - task_id={}, status={:?}, message={}",
                                        tool_execution.id,
                                        tool_execution.name,
                                        tool_execution.task_id,
                                        tool_execution.status,
                                        tool_execution.message
                                    );

                                    // Send as a tool_status notification directly
                                    let _ = rpc_server.send_notification(
                                        "tool_status",
                                        serde_json::json!({
                                            "type": "started",
                                            "execution": tool_execution
                                        }),
                                    );

                                    // Also send the legacy tool_execution event for backward compatibility
                                    let _ = rpc_server.event_sender().send((
                                        "tool_execution".to_string(),
                                        serde_json::json!({
                                            "task_id": task_id_clone2,
                                            "tool": tool_name,
                                            "message": tool_message,
                                            "status": status,
                                            "description": description,
                                            "file_path": file_path,
                                            "lines": lines,
                                            "timestamp": now
                                        }),
                                    ));
                                }
                            }
                        }

                        // Forward to main progress handler
                        let _ = progress_tx_clone.send(message.to_string());

                        // Log to stderr for debugging
                        eprintln!(
                            "{}",
                            format_log_with_color(LogLevel::Debug, &format!("Agent: {}", message))
                        );
                    }
                });
            });

            // Initialize the agent
            runtime.block_on(async { agent.initialize_with_api_key(api_key.clone()).await })?;

            // Execute the agent with the prompt
            let response = runtime.block_on(async { agent.execute(prompt).await })?;

            // Use a fixed tool count for now
            if let Some(task) = self.current_task_mut() {
                task.tool_count = 1; // Set a default tool count
            }

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
                        "Agent query completed, received approximately {} tokens",
                        estimated_tokens
                    )
                )
            );

            Ok(response)
        } else {
            // Execute the appropriate API call for non-agent mode
            let response = if model_name_lower.contains("claude") {
                // Use Anthropic API for Claude models
                runtime.block_on(async {
                    let client = crate::apis::anthropic::AnthropicClient::with_api_key(
                        api_key.clone(),
                        Some(model_file_name.clone()),
                    )?;

                    // Send progress update
                    let _ = progress_tx.send(format!("Sending request to {}", model_name));

                    client.complete(messages.clone(), options).await
                })?
            } else if model_name_lower.contains("gpt") {
                // Use OpenAI API for GPT models
                runtime.block_on(async {
                    let client = crate::apis::openai::OpenAIClient::with_api_key(
                        api_key.clone(),
                        Some(model_file_name.clone()),
                    )?;

                    // Send progress update
                    let _ = progress_tx.send(format!("Sending request to {}", model_name));

                    client.complete(messages.clone(), options).await
                })?
            } else if model_name_lower.contains("gemini") {
                // Use Gemini API for Gemini models
                runtime.block_on(async {
                    let client = crate::apis::gemini::GeminiClient::with_api_key(
                        api_key.clone(),
                        Some(model_file_name.clone()),
                    )?;

                    // Send progress update
                    let _ = progress_tx.send(format!("Sending request to {}", model_name));

                    client.complete(messages.clone(), options).await
                })?
            } else if model_name_lower.contains("local") {
                // Use Ollama API for local models
                runtime.block_on(async {
                    let client =
                        crate::apis::ollama::OllamaClient::new(Some(model_file_name.clone()))?;

                    // Send progress update
                    let _ = progress_tx.send(format!(
                        "Sending request to local model {}",
                        model_file_name
                    ));

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

    /// Start a new tool execution
    pub fn start_tool_execution(&mut self, name: &str) -> Option<String> {
        // Need a current task to track tool executions
        if let Some(task_id) = &self.current_task_id {
            // Create a new tool execution
            let tool_execution = ToolExecution::new(task_id, name);
            let tool_id = tool_execution.id.clone();

            // Store the tool execution
            self.tool_executions.insert(tool_id.clone(), tool_execution);

            // Increment the task's tool count
            if let Some(task) = self.current_task_mut() {
                task.add_tool_use();
            }

            // Send tool started notification
            if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                // More detailed logging
                eprintln!(
                    "Sending tool_status started notification for tool {}: {}",
                    name, tool_id
                );

                // Get the tool execution to send
                let tool_exec = self.tool_executions.get(&tool_id).cloned();

                if let Some(exec) = tool_exec {
                    let result = rpc_server.send_notification(
                        "tool_status",
                        serde_json::json!({
                            "type": "started",
                            "execution": exec
                        }),
                    );

                    if let Err(e) = result {
                        eprintln!("Error sending tool_status notification: {}", e);
                    }
                } else {
                    eprintln!("Tool execution not found for ID: {}", tool_id);
                }
            } else {
                eprintln!("No RPC server available to send tool_status notification");
            }

            Some(tool_id)
        } else {
            None
        }
    }

    /// Update tool execution progress
    pub fn update_tool_progress(
        &mut self,
        tool_id: &str,
        message: &str,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) {
        if let Some(tool) = self.tool_executions.get_mut(tool_id) {
            tool.update_progress(message);

            // Add any metadata if provided
            if let Some(meta) = metadata {
                for (key, value) in meta {
                    tool.add_metadata(&key, value);
                }
            }

            // Send progress notification
            if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                let _ = rpc_server.send_notification(
                    "tool_status",
                    serde_json::json!({
                        "type": "updated",
                        "execution": tool
                    }),
                );
            }
        }
    }

    /// Complete a tool execution
    pub fn complete_tool_execution(
        &mut self,
        tool_id: &str,
        message: &str,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) {
        if let Some(tool) = self.tool_executions.get_mut(tool_id) {
            tool.complete(message);

            // Add any metadata if provided
            if let Some(meta) = metadata {
                for (key, value) in meta {
                    tool.add_metadata(&key, value);
                }
            }

            // Send completion notification
            if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                let _ = rpc_server.send_notification(
                    "tool_status",
                    serde_json::json!({
                        "type": "updated",
                        "execution": tool
                    }),
                );
            }
        }
    }

    /// Mark a tool execution as failed
    pub fn fail_tool_execution(&mut self, tool_id: &str, error: &str) {
        if let Some(tool) = self.tool_executions.get_mut(tool_id) {
            tool.fail(error);

            // Send failure notification
            if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                let _ = rpc_server.send_notification(
                    "tool_status",
                    serde_json::json!({
                        "type": "updated",
                        "execution": tool
                    }),
                );
            }
        }
    }

    /// Clean up old completed tool executions (older than 10 minutes)
    pub fn cleanup_old_tool_executions(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let ten_minutes_ms = 10 * 60 * 1000;

        // Collect IDs of old completed tool executions
        let old_tool_ids: Vec<String> = self
            .tool_executions
            .iter()
            .filter(|(_, tool)| {
                if let Some(end_time) = tool.end_time {
                    // Keep if still running or completed less than 10 minutes ago
                    match tool.status {
                        ToolExecutionStatus::Running => false,
                        _ => now - end_time > ten_minutes_ms,
                    }
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Remove old tool executions
        for id in old_tool_ids {
            self.tool_executions.remove(&id);
        }
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
