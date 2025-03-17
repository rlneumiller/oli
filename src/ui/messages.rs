use crate::app::models::ModelManager;
use crate::app::permissions::PermissionHandler;
use crate::app::state::{App, AppState};
use crate::app::utils::Scrollable;
use anyhow::Result;

/// Initializes welcome messages for the setup screen
pub fn initialize_setup_messages(app: &mut App) {
    app.messages.extend(vec![
        "★ Welcome to OLI Assistant! ★".into(),
        "A terminal-based code assistant powered by local LLMs".into(),
        "".into(),
        "1. Select a model using Up/Down arrow keys".into(),
        "2. Press Enter to download and set up the selected model".into(),
        "3. After setup, you can chat with the assistant about code".into(),
        "".into(),
    ]);
}

/// Process a message from the model or agent
pub fn process_message(app: &mut App, msg: &str) -> Result<()> {
    if app.debug_messages {
        app.messages
            .push(format!("DEBUG: Processing message: {}", msg));
    }

    // Check for permission request message first - these are high priority
    if msg.starts_with("[permission_request]") {
        // Format is [permission_request]tool_name|tool_args
        let content = msg.strip_prefix("[permission_request]").unwrap_or("");
        let parts: Vec<&str> = content.splitn(2, '|').collect();

        if parts.len() == 2 {
            let tool_name = parts[0];
            let tool_args = parts[1];

            // Request permission for the tool
            app.request_tool_permission(tool_name, tool_args);
            return Ok(());
        }
    } else if msg.starts_with("progress:") {
        process_progress_message(app, msg);
    } else if msg.starts_with("status:") {
        process_status_message(app, msg);
    } else if msg.starts_with("download_started:") {
        process_download_started_message(app, msg);
    } else if msg == "download_complete" {
        process_download_complete_message(app)?;
    } else if msg == "api_key_needed" {
        process_api_key_needed_message(app);
    } else if msg == "setup_complete" {
        process_setup_complete_message(app);
    } else if msg == "setup_failed" {
        process_setup_failed_message(app);
    } else if msg.starts_with("error:") {
        process_error_message(app, msg);
    } else if msg.starts_with("retry:") {
        process_retry_message(app, msg);
    } else if is_tool_execution_message(msg) {
        process_tool_execution_message(app, msg);
    } else if is_ai_processing_message(msg) {
        process_ai_processing_message(app, msg);
    } else if is_tool_message(msg) {
        process_tool_message(app, msg);
    } else if is_tool_result_message(msg) {
        process_tool_result_message(app, msg);
    } else if is_success_message(msg) {
        process_success_message(app, msg);
    } else if msg.starts_with("Using tool") {
        process_using_tool_message(app, msg);
    } else if is_thinking_message(msg) {
        process_thinking_message(app, msg);
    } else if msg.starts_with("[permission]") {
        process_permission_message(app, msg);
    } else if msg == "Agent initialized successfully" {
        process_agent_initialized_message(app);
    } else if msg.starts_with("Failed to initialize agent") {
        process_agent_failure_message(app, msg);
    } else if is_completion_message(msg) {
        process_completion_message(app, msg);
    }

    Ok(())
}

// Helper functions for processing different message types
fn process_progress_message(app: &mut App, msg: &str) {
    // Make sure download_active is true whenever we receive progress
    app.download_active = true;

    let parts: Vec<&str> = msg.split(':').collect();
    if parts.len() >= 3 {
        if let (Ok(downloaded), Ok(total)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
            app.download_progress = Some((downloaded, total));
            // Only log progress occasionally to avoid flooding logs
            if downloaded % (5 * 1024 * 1024) < 100000 {
                // Log roughly every 5MB
                if app.debug_messages {
                    app.messages.push(format!(
                        "DEBUG: Download progress: {:.1}MB/{:.1}MB",
                        downloaded as f64 / 1_000_000.0,
                        total as f64 / 1_000_000.0
                    ));
                }
            }
        }
    }
}

fn process_status_message(app: &mut App, msg: &str) {
    // Status updates for the download process
    let status = msg.replacen("status:", "", 1);
    app.messages.push(format!("Status: {}", status));
}

fn process_download_started_message(app: &mut App, msg: &str) {
    app.download_active = true;
    let url = msg.replacen("download_started:", "", 1);
    app.messages.push(format!("Starting download from {}", url));
}

fn process_download_complete_message(app: &mut App) -> Result<()> {
    app.download_active = false;
    app.messages
        .push("Download completed! Loading model...".into());
    let model_path = app.model_path(&app.current_model().file_name)?;
    match app.load_model(&model_path) {
        Ok(()) => {
            // Directly transition to Chat state and set welcome message
            app.state = AppState::Chat;

            // Add the welcome message and help info in a cleaner format
            app.messages.clear(); // Clear setup messages for a clean chat window
            app.messages.push("★ Welcome to OLI assistant! ★".into());
            app.messages
                .push("Ready to code! Type /help for available commands".into());
            if let Some(cwd) = &app.current_working_dir {
                app.messages.push(format!("cwd: {}", cwd));
            }
            app.messages.push("".into());
        }
        Err(e) => {
            app.messages
                .push(format!("ERROR: Failed to load model: {}", e));
            app.state = AppState::Error(format!("Failed to load model: {}", e));
        }
    }
    Ok(())
}

fn process_api_key_needed_message(app: &mut App) {
    // Special case for when we need an API key
    app.messages
        .push("Please enter your Anthropic API key to use Claude 3.7...".into());
}

fn process_setup_complete_message(app: &mut App) {
    app.state = AppState::Chat;

    // Add the welcome message and help info in a cleaner format
    app.messages.clear(); // Clear setup messages for a clean chat window
    app.messages.push("★ Welcome to OLI assistant! ★".into());
    app.messages
        .push("Ready to code! Type /help for available commands".into());
    if let Some(cwd) = &app.current_working_dir {
        app.messages.push(format!("cwd: {}", cwd));
    }
    app.messages.push("".into());
}

fn process_setup_failed_message(app: &mut App) {
    app.messages
        .push("Setup failed. Check error messages above.".into());
}

fn process_error_message(app: &mut App, msg: &str) {
    let error_msg = msg.replacen("error:", "", 1);
    app.error_message = Some(error_msg.clone());
    app.state = AppState::Error(error_msg);
}

fn process_retry_message(app: &mut App, msg: &str) {
    app.messages.push(msg.replacen("retry:", "", 1));
}

fn is_tool_execution_message(msg: &str) -> bool {
    msg.starts_with("Executing tool") || msg.starts_with("Running tool")
}

fn process_tool_execution_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Show the raw tool execution messages to indicate tool usage during query
    app.messages.push(format!("⚙️ {}", msg));
    // Force immediate redraw to show tool usage in real-time
    app.auto_scroll_to_bottom();
}

fn is_ai_processing_message(msg: &str) -> bool {
    msg.starts_with("Sending request to AI")
        || msg.starts_with("Processing tool results")
        || msg.starts_with("[wait]")
}

fn process_ai_processing_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Handle AI operation with white circle - standardize all waiting operations
    if msg.starts_with("[wait]") {
        // Remove [wait] prefix and use the rest of the message
        let content = msg.strip_prefix("[wait] ").unwrap_or(msg);
        app.messages.push(content.to_string());
    } else {
        // Legacy format that needs the circle added
        app.messages.push(format!("⚪ {}", msg));
    }
    // Force immediate redraw to show AI thinking in real-time
    // Add timestamp to make this message "active" for a short period
    app.last_message_time = std::time::Instant::now();
    app.auto_scroll_to_bottom();
}

fn is_tool_message(msg: &str) -> bool {
    msg.starts_with("[tool]")
}

fn process_tool_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // This is a formatted tool operation, style it properly with green indicator
    if msg.starts_with("[tool] ⏺ ") {
        // Legacy format with black circle
        let content = msg.strip_prefix("[tool] ⏺ ").unwrap_or(msg);
        app.messages.push(format!("\x1b[32m⏺\x1b[0m {}", content)); // Green colored circle
    } else if msg.starts_with("[tool] 🔧") {
        // New format with wrench emoji - tool execution
        app.messages.push(msg.to_string());
    } else {
        // Generic tool message format
        let content = msg.strip_prefix("[tool] ").unwrap_or(msg);
        app.messages.push(format!("\x1b[32m⏺\x1b[0m {}", content));
    }
    // Update timestamp for animation effect
    app.last_message_time = std::time::Instant::now();
    // Force immediate redraw to show tool usage in real-time
    app.auto_scroll_to_bottom();
}

fn is_tool_result_message(msg: &str) -> bool {
    msg.starts_with("Tool result:")
}

fn process_tool_result_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Display tool results directly
    let content = msg.strip_prefix("Tool result:").unwrap_or(msg);
    app.messages
        .push(format!("\x1b[32m⏺\x1b[0m Tool result: {}", content));
}

fn is_success_message(msg: &str) -> bool {
    msg.starts_with("[success]")
}

fn process_success_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled and this isn't a final completion message
    if !app.show_intermediate_steps && !msg.contains("All tools executed successfully") {
        return;
    }

    // This is the tool result from execution
    let content = if msg.starts_with("[success] ⏺ ") {
        // Legacy format with black circle
        msg.strip_prefix("[success] ⏺ ").unwrap_or(msg)
    } else {
        // New formats
        msg.strip_prefix("[success] ").unwrap_or(msg)
    };

    // Check if this is a multi-line result with our tree structure format
    if content.contains("\n  ⎿") {
        // Extract the header and lines
        let parts: Vec<&str> = content.splitn(2, '\n').collect();
        let header = parts[0];

        // Display the tool execution with indented output
        app.messages.push(format!("\x1b[32m⏺\x1b[0m {}", header));
        // Update timestamp for animation effect
        app.last_message_time = std::time::Instant::now();

        if parts.len() > 1 {
            let lines = parts[1].lines().take(10); // Limit to 10 lines max
            for line in lines {
                app.messages.push(line.to_string()); // Already has indentation
                                                     // Small delay between adding each line for a typing effect
                std::thread::sleep(std::time::Duration::from_millis(50));
                // Don't try to force redraw - we'll let the main loop handle it
            }

            // If there are more lines, show a line count
            let total_lines = parts[1].lines().count();
            if total_lines > 10 {
                app.messages
                    .push(format!("  ... [{} more lines]", total_lines - 10));
            }
        }
    } else {
        // Simple single-line result
        app.messages.push(format!("\x1b[32m⏺\x1b[0m {}", content));
        // Update timestamp for animation effect
        app.last_message_time = std::time::Instant::now();
    }
    // Force immediate redraw to show tool results in real-time
    app.auto_scroll_to_bottom();
}

fn process_using_tool_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Show tool usage messages
    app.messages.push(format!("⚙️ {}", msg));
    app.auto_scroll_to_bottom();
}

fn is_thinking_message(msg: &str) -> bool {
    msg.starts_with("Thinking") || msg.contains("analyzing")
}

fn process_thinking_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Handle AI thinking process messages with white circle
    // The [thinking] prefix is already stripped by the caller
    app.messages.push(format!("⚪ {}", msg));
}

fn process_permission_message(app: &mut App, msg: &str) {
    // Permission-related messages are always shown
    app.messages.push(msg.to_string());
    app.auto_scroll_to_bottom();
}

fn process_agent_initialized_message(app: &mut App) {
    app.messages
        .push("⏺ Agent initialized and ready to use!".into());
}

fn process_agent_failure_message(app: &mut App, msg: &str) {
    app.messages.push(format!("[error] ❌ {}", msg));
    app.use_agent = false;
}

fn is_completion_message(msg: &str) -> bool {
    msg.contains("completed successfully") || msg.contains("done")
}

fn process_completion_message(app: &mut App, msg: &str) {
    app.messages.push(format!("⏺ {}", msg));
}
