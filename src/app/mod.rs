pub mod agent;
pub mod commands;
pub mod models;
pub mod permissions;
pub mod state;
pub mod utils;

use anyhow::{Context, Result};
use dotenv::dotenv;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Runtime;

// Re-exports
pub use agent::{determine_agent_model, determine_provider, AgentManager};
pub use commands::{get_available_commands, CommandHandler, SpecialCommand};
pub use models::ModelManager;
pub use permissions::{PendingToolExecution, PermissionHandler, ToolPermissionStatus};
pub use state::{App, AppState};
pub use utils::{ErrorHandler, Scrollable};

use crate::agent::core::{Agent, LLMProvider};
use crate::inference;
use crate::models::{get_available_models, ModelConfig};

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        // Load environment variables from .env file if present
        let _ = dotenv();

        // Create tokio runtime for async operations
        let tokio_runtime = Runtime::new().ok();

        // Get current working directory
        let current_working_dir = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string());

        Self {
            state: AppState::Setup,
            input: String::new(),
            messages: vec![],
            download_progress: None,
            selected_model: 0,
            available_models: get_available_models(),
            inference: None,
            download_active: false,
            error_message: None,
            debug_messages: false, // Default to debug messages off
            scroll_position: 0,
            last_query_time: std::time::Instant::now(),
            last_message_time: std::time::Instant::now(), // For animation effects
            use_agent: false,
            agent: None,
            tokio_runtime,
            agent_progress_rx: None,
            api_key: None,
            current_working_dir,
            // Initialize command-related fields
            command_mode: false,
            available_commands: get_available_commands(),
            selected_command: 0,
            show_command_menu: false,
            // Initialize tool permission-related fields
            permission_required: false,
            pending_tool: None,
            tool_permission_status: ToolPermissionStatus::Pending,
            tool_execution_in_progress: false,
            show_intermediate_steps: true, // Default to showing intermediate steps
            show_shortcuts_hint: true,     // Default to showing shortcut hints
            show_detailed_shortcuts: false, // Default to not showing detailed shortcuts
        }
    }
}

// Implement the various traits for App
impl CommandHandler for App {
    fn check_command_mode(&mut self) {
        // Track previous state
        let was_in_command_mode = self.command_mode;

        // Update command mode state
        self.command_mode = self.input.starts_with('/');
        self.show_command_menu = self.command_mode && !self.input.contains(' ');

        // Always reset the command selection in these cases:
        if self.command_mode {
            let filtered = self.filtered_commands();

            // Reset when:
            // 1. Just entered command mode (typed '/')
            // 2. Selection is out of bounds
            // 3. Input has changed significantly
            let should_reset = (self.input.len() == 1 && !was_in_command_mode)
                || (filtered.is_empty() || self.selected_command >= filtered.len());

            if should_reset {
                // Start from the beginning
                self.selected_command = 0;

                // Debug logging
                if self.debug_messages {
                    self.messages.push(format!(
                        "DEBUG: Reset command selection. Input: '{}', Commands: {}",
                        self.input,
                        filtered.len()
                    ));
                }
            }
        }
    }

    fn filtered_commands(&self) -> Vec<SpecialCommand> {
        if !self.command_mode || self.input.len() <= 1 {
            // Return all commands when just typing "/"
            return self.available_commands.clone();
        }

        // Filter commands that start with the input text
        self.available_commands
            .iter()
            .filter(|cmd| cmd.name.starts_with(&self.input))
            .cloned()
            .collect()
    }

    fn select_next_command(&mut self) {
        // Get filtered commands
        let filtered = self.filtered_commands();

        if self.show_command_menu && !filtered.is_empty() {
            // Store the number of commands
            let num_commands = filtered.len();

            // Always ensure we're in bounds and wrap properly
            if num_commands == 0 {
                return; // No commands available
            }

            // Ensure we're in bounds first
            self.selected_command = self.selected_command.min(num_commands - 1);

            // Then move forward one position with wraparound
            self.selected_command = (self.selected_command + 1) % num_commands;

            // Debug message
            if self.debug_messages {
                self.messages.push(format!(
                    "DEBUG: Selected command {} of {}",
                    self.selected_command + 1,
                    num_commands
                ));
            }
        }
    }

    fn select_prev_command(&mut self) {
        // Get filtered commands
        let filtered = self.filtered_commands();

        if self.show_command_menu && !filtered.is_empty() {
            // Store the number of commands
            let num_commands = filtered.len();

            // Always ensure we're in bounds and wrap properly
            if num_commands == 0 {
                return; // No commands available
            }

            // Ensure we're in bounds first
            self.selected_command = self.selected_command.min(num_commands - 1);

            // Calculate previous with wraparound
            self.selected_command = if self.selected_command == 0 {
                num_commands - 1 // Wrap to last command
            } else {
                self.selected_command - 1
            };

            // Debug message
            if self.debug_messages {
                self.messages.push(format!(
                    "DEBUG: Selected command {} of {}",
                    self.selected_command + 1,
                    num_commands
                ));
            }
        }
    }

    fn execute_command(&mut self) -> bool {
        if !self.command_mode {
            return false;
        }

        // Get the command to execute (either selected or entered)
        let command_to_execute = if self.show_command_menu {
            // Get the filtered commands
            let filtered = self.filtered_commands();
            if filtered.is_empty() {
                return false;
            }

            // Safely get a valid index into the filtered commands list
            let valid_index = self.selected_command.min(filtered.len() - 1);
            filtered[valid_index].name.clone()
        } else {
            self.input.clone()
        };

        // Execute the command
        match command_to_execute.as_str() {
            "/help" => {
                self.messages.push("Available commands:".into());
                for cmd in &self.available_commands {
                    self.messages
                        .push(format!("{} - {}", cmd.name, cmd.description));
                }
                self.messages.push("".into()); // Empty line for spacing
                true
            }
            "/clear" => {
                self.messages.clear();
                self.messages.push("Conversation history cleared.".into());
                self.scroll_position = 0;
                true
            }
            "/debug" => {
                // Toggle debug messages visibility
                self.debug_messages = !self.debug_messages;
                self.messages.push(format!(
                    "Debug messages {}.",
                    if self.debug_messages {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
                true
            }
            "/steps" => {
                // Toggle showing intermediate steps
                self.show_intermediate_steps = !self.show_intermediate_steps;
                self.messages.push(format!(
                    "Intermediate steps display {}.",
                    if self.show_intermediate_steps {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
                if self.show_intermediate_steps {
                    self.messages.push(
                        "Tool usage and intermediate operations will be shown as they happen."
                            .into(),
                    );
                } else {
                    self.messages.push(
                        "Only the final response will be shown without intermediate steps.".into(),
                    );
                }
                true
            }
            "/exit" => {
                self.state = AppState::Error("quit".into());
                true
            }
            _ => false,
        }
    }
}

impl Scrollable for App {
    fn scroll_up(&mut self, amount: usize) {
        if self.scroll_position > 0 {
            self.scroll_position = self.scroll_position.saturating_sub(amount);
        }
    }

    fn scroll_down(&mut self, amount: usize) {
        let max_scroll = self.messages.len().saturating_sub(10);
        if self.scroll_position < max_scroll {
            self.scroll_position = (self.scroll_position + amount).min(max_scroll);
        }
    }

    fn auto_scroll_to_bottom(&mut self) {
        // Calculate a better scroll position that ensures the latest messages are visible
        // We need to consider the actual height of the terminal window, but since that's
        // not directly available here, we use a conservative estimate to ensure we're
        // always showing the latest content

        // Aim to put the scroll position about 15-20 lines from the end
        // This is more reliable than the previous approach
        let max_scroll = self.messages.len().saturating_sub(5);
        self.scroll_position = max_scroll;

        // Mark that we've auto-scrolled so UI knows to maintain this position
        // even if multiple messages come in rapid succession
        self.messages.push("".to_string()); // Add empty line to ensure spacing
    }
}

impl ErrorHandler for App {
    fn handle_error(&mut self, message: String) {
        self.error_message = Some(message.clone());
        self.messages.push(format!("Error: {}", message));
    }
}

impl ModelManager for App {
    fn current_model(&self) -> &ModelConfig {
        &self.available_models[self.selected_model]
    }

    fn select_next_model(&mut self) {
        self.selected_model = (self.selected_model + 1) % self.available_models.len();
    }

    fn select_prev_model(&mut self) {
        self.selected_model = if self.selected_model == 0 {
            self.available_models.len() - 1
        } else {
            self.selected_model - 1
        };
    }

    fn models_dir() -> Result<PathBuf> {
        let models_dir = dirs::home_dir()
            .context("Failed to find home directory")?
            .join(".oli")
            .join("models");

        // Create the models directory if it doesn't exist
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir).context("Failed to create models directory")?;
        }

        Ok(models_dir)
    }

    fn model_path(&self, model_name: &str) -> Result<PathBuf> {
        let models_dir = Self::models_dir()?;
        Ok(models_dir.join(model_name))
    }

    fn verify_model(&self, path: &Path) -> Result<()> {
        // Check if file exists and has a reasonable size
        let metadata = std::fs::metadata(path)?;
        if metadata.len() < 1000 {
            anyhow::bail!(
                "File too small to be a valid model ({}bytes)",
                metadata.len()
            );
        }

        // Read first few bytes to check the file format
        let mut file = File::open(path)?;
        let mut header = [0u8; 8];

        if file.read_exact(&mut header[0..4]).is_err() {
            anyhow::bail!("Failed to read header - file may be corrupted");
        }

        // Check for GGUF format
        if &header[0..4] == b"GGUF" {
            return Ok(());
        }

        // Check for GGML format (older models)
        if &header[0..4] == b"GGML" {
            return Ok(());
        }

        // Read first ~100 bytes to check for HTML error pages
        let mut start_bytes = vec![0u8; 100];
        file.seek(SeekFrom::Start(0))?; // Reset to beginning of file
        let n = file.read(&mut start_bytes)?;
        start_bytes.truncate(n);

        let start_text = String::from_utf8_lossy(&start_bytes);

        // Check if the file is actually an HTML error page
        if start_text.contains("<html")
            || start_text.contains("<!DOCTYPE")
            || start_text.contains("<HTML")
            || start_text.contains("<?xml")
        {
            anyhow::bail!("Received HTML page instead of model file");
        }

        // If the file is large enough, assume it's a valid model despite unknown format
        if metadata.len() > 100 * 1024 * 1024 {
            // > 100MB
            return Ok(());
        }

        anyhow::bail!(
            "Unrecognized model format (magic: {:?} or '{}')",
            &header[0..4],
            String::from_utf8_lossy(&header[0..4])
        )
    }

    fn verify_static(path: &Path) -> Result<()> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 8]; // Read more bytes to check different formats

        if let Err(e) = file.read_exact(&mut header[0..4]) {
            anyhow::bail!("Failed to read file header: {}", e);
        }

        // Check for GGUF format (standard)
        if &header[0..4] == b"GGUF" {
            return Ok(());
        }

        // Check for GGML format (older models)
        if &header[0..4] == b"GGML" {
            return Ok(());
        }

        // Try to read a bit more to check for binary format
        if let Ok(()) = file.read_exact(&mut header[4..8]) {
            // Some binary signatures to check
            if header[0] == 0x80
                && header[1] <= 0x02
                && (header[2] == 0x00 || header[2] == 0x01)
                && header[3] == 0x00
            {
                return Ok(());
            }
        }

        // Get the file size to see if it's reasonable for an LLM
        if let Ok(metadata) = std::fs::metadata(path) {
            let size_mb = metadata.len() / (1024 * 1024);
            // If file is reasonably large (> 100MB), accept it despite unknown format
            if size_mb > 100 {
                return Ok(());
            }
        }

        // If we get here, the file format wasn't recognized
        let magic_str = String::from_utf8_lossy(&header[0..4]);
        anyhow::bail!(
            "Unknown model format (magic: {:?} or '{}')",
            &header[0..4],
            magic_str
        )
    }

    fn get_agent_model(&self) -> Option<String> {
        // Return the appropriate model ID based on the current selected model
        let model_name = self.current_model().name.as_str();
        let has_api_key = std::env::var("ANTHROPIC_API_KEY").is_ok()
            || std::env::var("OPENAI_API_KEY").is_ok()
            || self.api_key.is_some();

        agent::determine_agent_model(model_name, has_api_key)
    }

    fn load_model(&mut self, model_path: &Path) -> Result<()> {
        if self.debug_messages {
            self.messages
                .push(format!("DEBUG: Loading model from {:?}", model_path));
            self.messages.push(format!(
                "DEBUG: Using {} GPU layers",
                self.current_model().n_gpu_layers
            ));
        }

        let n_gpu_layers = self.current_model().n_gpu_layers;
        let model_config = self.current_model();

        // Check if the model supports agent capabilities
        let supports_agent = model_config
            .agentic_capabilities
            .as_ref()
            .map(|caps| !caps.is_empty())
            .unwrap_or(false);

        // Try setting up agent if supported
        if supports_agent {
            if let Err(e) = self.setup_agent() {
                self.messages.push(format!(
                    "WARNING: Failed to initialize agent capabilities: {}",
                    e
                ));
                self.use_agent = false;
            } else if self.use_agent {
                self.messages.push(
                    "💡 Agent capabilities enabled! You can now use advanced code tasks.".into(),
                );
                self.messages
                    .push("Try asking about files, editing code, or running commands.".into());
                self.download_active = false;
                self.state = AppState::Chat;

                // If agent is successfully set up, we can skip loading the local model
                if self.agent.is_some() {
                    return Ok(());
                }
            }
        }

        // Fall back to loading local model
        match inference::ModelSession::new(model_path, n_gpu_layers) {
            Ok(inference) => {
                self.inference = Some(inference);
                self.messages.push("Model loaded successfully!".into());
                self.messages
                    .push("You can now ask questions about coding tasks.".into());
                self.messages.push("Try asking about specific programming concepts, debugging help, or code explanations.".into());
                self.download_active = false;
                self.state = AppState::Chat;
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to load model: {}", e);
                self.messages.push(format!("ERROR: {}", error_msg));

                // Add more detailed diagnostic info
                if self.debug_messages {
                    self.messages
                        .push(format!("DEBUG: Model file exists: {}", model_path.exists()));
                    if let Ok(metadata) = std::fs::metadata(model_path) {
                        self.messages
                            .push(format!("DEBUG: Model file size: {} bytes", metadata.len()));
                    }
                }

                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    fn setup_models(&mut self, tx: mpsc::Sender<String>) -> Result<()> {
        if self.debug_messages {
            self.messages.push("DEBUG: setup_models called".into());
        }

        self.error_message = None;

        let model_name = self.current_model().name.clone();
        let model_file_name = self.current_model().file_name.clone();
        let model_primary_url = self.current_model().primary_url.clone();
        let model_fallback_url = self.current_model().fallback_url.clone();
        let model_size = self.current_model().size_gb;

        self.messages
            .push(format!("Setting up model: {}", model_name));

        // For cloud-based models (size 0.0)
        if model_size == 0.0 {
            if self.debug_messages {
                self.messages
                    .push(format!("DEBUG: Setting up {}", model_name));
            }

            // Check if we need to ask for API key based on the selected model
            let needs_api_key = match model_name.as_str() {
                "GPT-4o" => std::env::var("OPENAI_API_KEY").is_err() && self.api_key.is_none(),
                "Claude 3.7 Sonnet" => {
                    std::env::var("ANTHROPIC_API_KEY").is_err() && self.api_key.is_none()
                }
                _ => true, // Default to requiring API key
            };

            if needs_api_key {
                // Transition to API key input state
                self.state = AppState::ApiKeyInput;
                self.input.clear();
                tx.send("api_key_needed".into())?;
                return Ok(());
            }

            // Setup agent with the appropriate model
            if let Err(e) = self.setup_agent() {
                self.handle_error(format!("Failed to setup {}: {}", model_name, e));
                tx.send("setup_failed".into())?;
                return Ok(());
            }

            // If agent is successfully set up, we're done
            if self.use_agent && self.agent.is_some() {
                tx.send("setup_complete".into())?;
                return Ok(());
            } else {
                let provider_name = match model_name.as_str() {
                    "GPT-4o" => "OpenAI",
                    _ => "Anthropic",
                };
                self.handle_error(format!("{} API key not found or is invalid", provider_name));
                tx.send("setup_failed".into())?;
                return Ok(());
            }
        }

        // For local models, continue with the normal setup process
        // Initialize download state for local models
        self.download_active = true;

        // Get the path for the selected model
        let model_path = self.model_path(&model_file_name)?;
        if model_path.exists() {
            if self.debug_messages {
                self.messages
                    .push(format!("DEBUG: Model file exists at {:?}", model_path));
            }

            match self.verify_model(&model_path) {
                Ok(()) => match self.load_model(&model_path) {
                    Ok(()) => {
                        if self.debug_messages {
                            self.messages
                                .push("DEBUG: Model loaded successfully".into());
                        }
                        tx.send("setup_complete".into())?;
                    }
                    Err(e) => {
                        self.handle_error(format!("Failed to load model: {}", e));
                        tx.send("setup_failed".into())?;
                    }
                },
                Err(e) => {
                    self.handle_error(format!("Invalid model file: {}", e));
                    std::fs::remove_file(&model_path).ok();
                    self.download_active = true;
                    self.messages
                        .push("Starting download after validation failure...".into());
                    self.download_model_with_path(
                        tx.clone(),
                        &model_path,
                        &model_primary_url,
                        &model_fallback_url,
                    )?;
                }
            }
            return Ok(());
        }

        if self.debug_messages {
            self.messages
                .push("DEBUG: Model file does not exist, downloading...".to_string());
        }

        self.download_active = true;
        self.messages
            .push(format!("Starting download of {}...", model_name));
        self.download_model_with_path(tx, &model_path, &model_primary_url, &model_fallback_url)
    }

    fn download_model_with_path(
        &mut self,
        tx: mpsc::Sender<String>,
        path: &Path,
        primary_url: &str,
        fallback_url: &str,
    ) -> Result<()> {
        if self.debug_messages {
            self.messages
                .push(format!("DEBUG: Downloading to {:?}", path));
            self.messages
                .push(format!("DEBUG: download_active={}", self.download_active));
        }
        self.download_file(primary_url, fallback_url, path, tx)
    }

    fn download_file(
        &mut self,
        primary_url: &str,
        fallback_url: &str,
        path: &Path,
        tx: mpsc::Sender<String>,
    ) -> Result<()> {
        let primary_url = primary_url.to_string();
        let fallback_url = fallback_url.to_string();
        let path = path.to_path_buf();
        let tx_clone = tx.clone();

        // Ensure download_active is set to true
        self.download_active = true;

        std::thread::spawn(move || {
            let download_result = {
                match Self::attempt_download(&primary_url, &path, &tx_clone) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        tx_clone
                            .send(format!("retry:First download attempt failed: {}", e))
                            .ok();

                        match Self::attempt_download(&fallback_url, &path, &tx_clone) {
                            Ok(()) => Ok(()),
                            Err(e2) => Err(format!(
                                "Both download attempts failed. Primary: {}, Fallback: {}",
                                e, e2
                            )),
                        }
                    }
                }
            };

            match download_result {
                Ok(()) => {
                    // Send a success message
                    tx_clone
                        .send("status:Download successful, verifying file...".into())
                        .unwrap();

                    // Verify after download completes
                    match Self::verify_static(&path) {
                        Ok(()) => {
                            tx_clone
                                .send("status:File verified successfully".into())
                                .unwrap();
                            tx_clone.send("download_complete".into()).unwrap()
                        }
                        Err(e) => tx_clone.send(format!("error:{}", e)).unwrap(),
                    }
                }
                Err(e) => tx_clone.send(format!("error:{}", e)).unwrap(),
            }
        });

        Ok(())
    }

    fn attempt_download(url: &str, path: &Path, tx: &mpsc::Sender<String>) -> Result<(), String> {
        // Send an initial message to indicate download is starting
        tx.send(format!("download_started:{}", url))
            .map_err(|e| format!("Channel error: {}", e))?;

        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(Duration::from_secs(300)) // Longer timeout for large files (5 min)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Client build failed: {}", e))?;

        // Notify about connection attempt
        tx.send(format!("status:Connecting to {}...", url))
            .map_err(|e| format!("Channel error: {}", e))?;

        let mut response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "*/*")
            .send()
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {} for URL {}", response.status(), url));
        }

        // Get the content length to track progress
        let total_size = response.content_length().unwrap_or(0);
        tx.send(format!(
            "status:Downloading {}MB file...",
            total_size / 1_000_000
        ))
        .map_err(|e| format!("Channel error: {}", e))?;

        let mut file = File::create(path).map_err(|e| format!("File creation failed: {}", e))?;

        // Initial progress
        tx.send(format!("progress:0:{}", total_size))
            .map_err(|e| format!("Channel error: {}", e))?;

        // Create a buffer for reading chunks
        let mut buffer = [0; 8192]; // 8KB buffer
        let mut downloaded: u64 = 0;
        let mut last_progress_time = std::time::Instant::now();

        // Read and write in chunks to show progress
        loop {
            match response.read(&mut buffer) {
                Ok(0) => break, // End of file
                Ok(n) => {
                    file.write_all(&buffer[..n])
                        .map_err(|e| format!("Write error: {}", e))?;

                    downloaded += n as u64;

                    // Update progress at most every 500ms to avoid flooding
                    let now = std::time::Instant::now();
                    if now.duration_since(last_progress_time).as_millis() > 500 {
                        tx.send(format!("progress:{}:{}", downloaded, total_size))
                            .map_err(|e| format!("Channel error: {}", e))?;
                        last_progress_time = now;
                    }
                }
                Err(e) => return Err(format!("Download error: {}", e)),
            }
        }

        // Final progress update
        tx.send(format!("progress:{}:{}", downloaded, total_size))
            .map_err(|e| format!("Channel error: {}", e))?;

        // Ensure file is written to disk
        file.sync_all()
            .map_err(|e| format!("File sync error: {}", e))?;

        Ok(())
    }
}

impl PermissionHandler for App {
    fn requires_permission(&self, tool_name: &str) -> bool {
        // Tools that require permission for potentially destructive operations
        match tool_name {
            "Edit" | "Replace" | "NotebookEditCell" => true, // File modification
            "Bash" => true,                                  // Shell commands (may be destructive)
            // Add other tools that require permission here
            _ => false, // Other tools don't require permission
        }
    }

    fn request_tool_permission(&mut self, tool_name: &str, args: &str) -> ToolPermissionStatus {
        // If permission is not required for this tool, auto-grant
        if !self.requires_permission(tool_name) {
            return ToolPermissionStatus::Granted;
        }

        // Create a user-friendly description of what the tool will do
        let description = match tool_name {
            "Edit" => {
                if let Some(file_path) = self.extract_argument(args, "file_path") {
                    format!("Modify file '{}'", file_path)
                } else {
                    "Edit a file".to_string()
                }
            }
            "Replace" => {
                if let Some(file_path) = self.extract_argument(args, "file_path") {
                    format!("Overwrite file '{}'", file_path)
                } else {
                    "Replace a file".to_string()
                }
            }
            "NotebookEditCell" => {
                if let Some(notebook_path) = self.extract_argument(args, "notebook_path") {
                    format!("Edit Jupyter notebook '{}'", notebook_path)
                } else {
                    "Edit a Jupyter notebook".to_string()
                }
            }
            "Bash" => {
                if let Some(command) = self.extract_argument(args, "command") {
                    format!("Execute command: '{}'", command)
                } else {
                    "Execute a shell command".to_string()
                }
            }
            _ => format!("Execute tool: {}", tool_name),
        };

        // Create a message for display
        let display_message = format!(
            "[permission] ⚠️ Permission required: {} - Press 'y' to allow or 'n' to deny",
            description
        );

        // Set up the permission request
        self.permission_required = true;
        self.pending_tool = Some(PendingToolExecution {
            tool_name: tool_name.to_string(),
            tool_args: args.to_string(),
            description: description.clone(),
        });
        self.tool_permission_status = ToolPermissionStatus::Pending;

        // Add a message to indicate permission is needed
        self.messages.push(display_message);
        self.auto_scroll_to_bottom();

        // Return pending status - UI will handle getting actual permission
        ToolPermissionStatus::Pending
    }

    fn handle_permission_response(&mut self, granted: bool) {
        if granted {
            self.tool_permission_status = ToolPermissionStatus::Granted;
            self.messages
                .push("[permission] ✅ Permission granted, executing tool...".to_string());
        } else {
            self.tool_permission_status = ToolPermissionStatus::Denied;
            self.messages
                .push("[permission] ❌ Permission denied, skipping tool execution".to_string());
        }
        self.auto_scroll_to_bottom();
    }

    fn extract_argument(&self, args: &str, arg_name: &str) -> Option<String> {
        // Simple parsing of JSON-like string to extract a specific argument
        if let Some(start_idx) = args.find(&format!("\"{}\":", arg_name)) {
            let value_start = args[start_idx..].find(":").map(|i| start_idx + i + 1)?;
            let value_text = args[value_start..].trim();

            // Check if value is a quoted string
            if let Some(stripped) = value_text.strip_prefix("\"") {
                let end_idx = stripped.find("\"").map(|i| value_start + i + 1)?;
                Some(value_text[1..end_idx].to_string())
            } else {
                // Non-string value - try to extract until comma or closing brace
                let end_chars = [',', '}'];
                let end_idx = end_chars
                    .iter()
                    .filter_map(|c| value_text.find(*c))
                    .min()
                    .map(|i| value_start + i)?;
                Some(value_text[..end_idx - value_start].trim().to_string())
            }
        } else {
            None
        }
    }

    fn requires_permission_check(&self) -> bool {
        true // Default to requiring permission for risky operations
    }
}

impl AgentManager for App {
    fn setup_agent(&mut self) -> Result<()> {
        // Check if API keys are available either from env vars or from user input
        let has_anthropic_key =
            std::env::var("ANTHROPIC_API_KEY").is_ok() || self.api_key.is_some();
        let has_openai_key = std::env::var("OPENAI_API_KEY").is_ok() || self.api_key.is_some();

        // Determine appropriate provider based on the selected model
        let provider = match agent::determine_provider(
            self.current_model().name.as_str(),
            has_anthropic_key,
            has_openai_key,
        ) {
            Some(provider) => provider,
            None => {
                // No valid provider found
                self.messages.push(
                    "No API key found for any provider. Agent features will be disabled.".into(),
                );
                self.messages.push("To enable agent features, set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable.".into());
                self.use_agent = false;
                return Ok(());
            }
        };

        // Create progress channel
        let (tx, rx) = mpsc::channel();
        self.agent_progress_rx = Some(rx);

        // Create the agent with API key if provided by user
        let mut agent = if let Some(api_key) = &self.api_key {
            Agent::new_with_api_key(provider.clone(), api_key.clone())
        } else {
            Agent::new(provider.clone())
        };

        // Add model if specified
        if let Some(model) = self.get_agent_model() {
            agent = agent.with_model(model);
        }

        // Initialize agent in the tokio runtime
        if let Some(runtime) = &self.tokio_runtime {
            runtime.block_on(async {
                let result = if let Some(api_key) = self.api_key.clone() {
                    // If we have a direct API key, use it (handles both user-input and env var)
                    agent.initialize_with_api_key(api_key).await
                } else {
                    // Otherwise try to initialize from environment variables
                    agent.initialize().await
                };

                if let Err(e) = result {
                    tx.send(format!("Failed to initialize agent: {}", e))
                        .unwrap();
                    return;
                }
                tx.send("Agent initialized successfully".to_string())
                    .unwrap();
            });

            self.agent = Some(agent);
            self.use_agent = true;

            // Show provider-specific message
            match provider {
                LLMProvider::Anthropic => {
                    self.messages
                        .push("Claude 3.7 Sonnet agent capabilities enabled!".into());
                }
                LLMProvider::OpenAI => {
                    self.messages
                        .push("GPT-4o agent capabilities enabled!".into());
                }
            }
        } else {
            self.messages
                .push("Failed to create async runtime. Agent features will be disabled.".into());
            self.use_agent = false;
        }

        Ok(())
    }

    fn query_model(&mut self, prompt: &str) -> Result<String> {
        if self.debug_messages {
            self.messages.push(format!(
                "DEBUG: Querying with: {}",
                if prompt.len() > 50 {
                    format!("{}...", &prompt[..50])
                } else {
                    prompt.to_string()
                }
            ));
        }

        // Try using agent if enabled
        if self.use_agent && self.agent.is_some() {
            return self.query_with_agent(prompt);
        }

        // Fall back to local model if agent is not available
        // Check if the model is loaded
        match self.inference.as_mut() {
            Some(inference) => {
                // Attempt to generate a response
                match inference.generate(prompt) {
                    Ok(response) => {
                        // Successful response
                        if self.debug_messages {
                            self.messages.push(format!(
                                "DEBUG: Generated response of {} characters",
                                response.len()
                            ));
                        }
                        Ok(response)
                    }
                    Err(e) => {
                        // Handle generation error
                        let error_msg = format!("Error generating response: {}", e);
                        self.messages.push(format!("ERROR: {}", error_msg));
                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            }
            None => {
                // Model not loaded
                let error_msg = "Model not loaded".to_string();
                self.messages.push(format!("ERROR: {}", error_msg));
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    fn query_with_agent(&mut self, prompt: &str) -> Result<String> {
        // Make sure we have a tokio runtime
        let runtime = match &self.tokio_runtime {
            Some(rt) => rt,
            None => return Err(anyhow::anyhow!("Async runtime not available")),
        };

        // Make sure we have an agent
        let agent = match &self.agent {
            Some(agent) => agent,
            None => return Err(anyhow::anyhow!("Agent not initialized")),
        };

        // Create a progress channel
        let (progress_tx, progress_rx) = mpsc::channel();
        self.agent_progress_rx = Some(progress_rx);

        // Add a message about starting to process the query
        self.messages
            .push("[thinking] Analyzing your query and preparing to use tools if needed...".into());
        // Force immediate update to show thinking message
        self.messages.push("_AUTO_SCROLL_".to_string());

        // Set tool execution flag
        self.tool_execution_in_progress = true;

        // Copy the agent and execute the query
        let agent_clone = agent.clone();
        let prompt_clone = prompt.to_string();

        // Process this as a background task in the tokio runtime
        let (response_tx, response_rx) = mpsc::channel();

        // Need to pass app state for tool permission checks
        let app_permission_required = self.requires_permission_check();

        runtime.spawn(async move {
            // Set up the agent with progress sender
            let (tokio_progress_tx, mut tokio_progress_rx) = tokio::sync::mpsc::channel(100);
            let agent_with_progress = agent_clone.with_progress_sender(tokio_progress_tx);

            // Create a channel for the response
            let (final_response_tx, final_response_rx) = tokio::sync::oneshot::channel();

            // Execute the query in a separate task
            tokio::spawn(async move {
                // Execute the actual query in background
                match agent_with_progress.execute(&prompt_clone).await {
                    Ok(response) => {
                        // Process response format
                        let processed_response =
                            if response.trim().starts_with("{") && response.trim().ends_with("}") {
                                // If it's JSON, ensure it's properly formatted
                                match serde_json::from_str::<serde_json::Value>(&response) {
                                    Ok(json) => {
                                        if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                                            pretty
                                        } else {
                                            response
                                        }
                                    }
                                    Err(_) => response,
                                }
                            } else {
                                response
                            };

                        // Signal that we're in the final response phase - but can't access progress_tx from here
                        // We'll handle the final message in the outer scope
                        // Send final response through the oneshot channel
                        let _ = final_response_tx.send(Ok(processed_response));
                    }
                    Err(e) => {
                        // Send error through the oneshot channel
                        let _ = final_response_tx.send(Err(e));
                    }
                }
            });

            // Forward progress messages in real-time while waiting for the final response
            // Need to clone the progress sender for use in multiple places
            let error_progress_tx = progress_tx.clone();
            let forwarder_progress_tx = progress_tx.clone();

            // Create a separate task to forward progress messages (don't need to track the handle)
            let _progress_forwarder = tokio::spawn(async move {
                while let Some(msg) = tokio_progress_rx.recv().await {
                    // Check for tool execution messages that require permission
                    if app_permission_required
                        && (msg.contains("Using tool: Edit")
                            || msg.contains("Using tool: Replace")
                            || msg.contains("Using tool: Bash")
                            || msg.contains("Using tool: NotebookEditCell"))
                    {
                        // Extract tool name and args
                        if let Some(tool_info) = msg.strip_prefix("Using tool: ") {
                            let parts: Vec<&str> = tool_info.splitn(2, " with args: ").collect();
                            if parts.len() == 2 {
                                let tool_name = parts[0];
                                let tool_args = parts[1];

                                // Send special permission request message
                                let _ = forwarder_progress_tx.send(format!(
                                    "[permission_request]{}|{}",
                                    tool_name, tool_args
                                ));

                                // Add auto-scroll flag to ensure the permission dialog shows
                                let _ = forwarder_progress_tx.send("_AUTO_SCROLL_".to_string());

                                // Wait a bit to allow UI to process the permission request
                                // This is not ideal but works as a simple solution
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                        }
                    }

                    // For each progress message, add an auto-scroll marker to ensure the UI updates
                    let _ = forwarder_progress_tx.send(msg);
                    // Add auto-scroll flag to ensure the UI updates in real-time
                    let _ = forwarder_progress_tx.send("_AUTO_SCROLL_".to_string());
                }
            });

            // Wait for the final response
            match final_response_rx.await {
                Ok(Ok(response)) => {
                    // Signal that we're finalizing the response
                    let _ = progress_tx.send("[wait] ⚪ Finalizing response...".to_string());
                    // Signal that the tool executions are complete
                    let _ =
                        progress_tx.send("[success] ⏺ All tools executed successfully".to_string());
                    // Send the final response
                    let _ = response_tx.send(Ok(response));
                }
                Ok(Err(e)) => {
                    // Send error message using the cloned sender
                    let _ = error_progress_tx
                        .send(format!("[error] ❌ Error during processing: {}", e));
                    let _ = response_tx.send(Err(e));
                }
                Err(_) => {
                    // Channel closed unexpectedly
                    let _ = response_tx.send(Err(anyhow::anyhow!(
                        "Agent processing channel closed unexpectedly"
                    )));
                }
            }

            // No need to explicitly abort, the task will end when the tokio runtime is dropped
        });

        // Wait for the response with a timeout (2 minutes) and return the final result
        let result = response_rx.recv_timeout(Duration::from_secs(120))?;

        // Clear tool execution state
        self.tool_execution_in_progress = false;
        self.permission_required = false;
        self.pending_tool = None;

        result
    }
}
