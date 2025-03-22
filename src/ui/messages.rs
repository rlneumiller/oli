use crate::app::permissions::PermissionHandler;
use crate::app::state::{App, AppState};
use crate::app::utils::Scrollable;
use anyhow::Result;
use std::time::Instant;

/// Message type enum for better categorization
#[derive(Debug, PartialEq)]
enum MessageType {
    Setup,
    User,
    Tool,
    ToolResult,
    Status,
    Error,
    Success,
    Permission,
    Debug,
    Info,
    Thinking,
    Unknown,
}

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
    // Add debug message if debug is enabled
    if app.debug_messages {
        app.messages
            .push(format!("DEBUG: Processing message: {}", msg));
    }

    // Determine the message type
    let msg_type = determine_message_type(msg);

    // Process based on message type
    match msg_type {
        MessageType::Permission => process_permission_related(app, msg),
        MessageType::Setup => process_setup_related(app, msg)?,
        MessageType::Tool => process_tool_message(app, msg),
        MessageType::ToolResult => process_tool_result_message(app, msg),
        MessageType::Success => process_success_message(app, msg),
        MessageType::Thinking => process_thinking_message(app, msg),
        MessageType::Error => process_error_message(app, msg),
        MessageType::Status => add_status_message(app, msg),
        MessageType::Debug => {
            // Only show debug messages in debug mode
            if app.debug_messages {
                app.messages.push(msg.to_string());
            }
        }
        MessageType::User => app.messages.push(msg.to_string()),
        MessageType::Info => app.messages.push(msg.to_string()),
        MessageType::Unknown => {
            // Just add the message without special formatting
            app.messages.push(msg.to_string());
        }
    }

    Ok(())
}

/// Determine the type of message for better processing
fn determine_message_type(msg: &str) -> MessageType {
    if msg.starts_with("[permission_request]") || msg.starts_with("[permission]") {
        MessageType::Permission
    } else if msg.starts_with("progress:")
        || msg.starts_with("status:")
        || msg.starts_with("download_started:")
        || msg == "download_complete"
        || msg == "api_key_needed"
        || msg == "setup_complete"
        || msg == "setup_failed"
    {
        MessageType::Setup
    } else if msg.starts_with("DEBUG:") {
        MessageType::Debug
    } else if msg.starts_with("> ") {
        MessageType::User
    } else if msg.starts_with("Error:")
        || msg.starts_with("ERROR:")
        || msg.starts_with("error:")
        || msg.starts_with("[error]")
    {
        MessageType::Error
    } else if msg.starts_with("Status:") {
        MessageType::Status
    } else if is_tool_message(msg)
        || msg.starts_with("Using tool")
        || is_tool_execution_message(msg)
    {
        MessageType::Tool
    } else if is_tool_result_message(msg) {
        MessageType::ToolResult
    } else if is_success_message(msg) {
        MessageType::Success
    } else if is_thinking_message(msg) {
        MessageType::Thinking
    } else if msg.starts_with("★") || msg.starts_with("Ready to code") {
        MessageType::Info
    } else {
        MessageType::Unknown
    }
}

/// Process permission-related messages
fn process_permission_related(app: &mut App, msg: &str) {
    if msg.starts_with("[permission_request]") {
        // Format is [permission_request]tool_name|tool_args
        if let Some(content) = msg.strip_prefix("[permission_request]") {
            let parts: Vec<&str> = content.splitn(2, '|').collect();

            if parts.len() == 2 {
                let tool_name = parts[0];
                let tool_args = parts[1];

                // Request permission for the tool
                app.request_tool_permission(tool_name, tool_args);
            }
        }
    } else {
        // Regular permission message
        app.messages.push(msg.to_string());
        app.auto_scroll_to_bottom();
    }
}

/// Process setup-related messages
fn process_setup_related(app: &mut App, msg: &str) -> Result<()> {
    if msg.starts_with("progress:") {
        let content = msg.replacen("progress:", "", 1);
        app.messages.push(format!("Progress: {}", content));
    } else if msg.starts_with("status:") {
        let status = msg.replacen("status:", "", 1);
        app.messages.push(format!("Status: {}", status));
    } else if msg.starts_with("download_started:") {
        let url = msg.replacen("download_started:", "", 1);
        app.messages.push(format!("Setup starting from {}", url));
    } else if msg == "download_complete" {
        app.messages
            .push("Setup completed! Loading model...".into());

        // Transition to Chat state
        app.state = AppState::Chat;

        // Add welcome message
        app.messages.push("★ Welcome to OLI assistant! ★".into());
        app.messages
            .push("Ready to code! Type /help for available commands".into());

        if let Some(cwd) = &app.current_working_dir {
            app.messages.push(format!("cwd: {}", cwd));
        }
        app.messages.push("".into());
    } else if msg == "api_key_needed" {
        app.messages
            .push("Please enter your Anthropic API key to use Claude 3.7...".into());
    } else if msg == "setup_complete" {
        app.state = AppState::Chat;

        // Clean welcome messages
        app.messages.clear();
        app.messages.push("★ Welcome to OLI assistant! ★".into());
        app.messages
            .push("Ready to code! Type /help for available commands".into());

        if let Some(cwd) = &app.current_working_dir {
            app.messages.push(format!("cwd: {}", cwd));
        }
        app.messages.push("".into());
    } else if msg == "setup_failed" {
        app.messages
            .push("Setup failed. Check error messages above.".into());
    } else if msg.starts_with("retry:") {
        app.messages.push(msg.replacen("retry:", "", 1));
    }

    Ok(())
}

/// Add a status message
fn add_status_message(app: &mut App, msg: &str) {
    app.messages.push(msg.to_string());
}

/// Process error messages
fn process_error_message(app: &mut App, msg: &str) {
    let error_content = if msg.starts_with("error:") {
        let error_msg = msg.replacen("error:", "", 1);
        app.error_message = Some(error_msg.clone());
        app.state = AppState::Error(error_msg.clone());
        error_msg
    } else if msg.starts_with("[error] ") {
        msg.replacen("[error] ", "", 1)
    } else {
        msg.to_string()
    };

    app.messages.push(format!("Error: {}", error_content));
}

/// Check if a message is a tool execution message
fn is_tool_execution_message(msg: &str) -> bool {
    msg.starts_with("Executing tool") || msg.starts_with("Running tool")
}

/// Process tool execution messages
fn process_tool_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Handle the different tool message formats based on status
    if msg.starts_with("⏺ [completed]") {
        // Tool completed (will be styled as green in UI)
        let content = msg.strip_prefix("⏺ [completed]").unwrap_or(msg).trim();
        app.messages.push(format!("⏺ {}", content));
    } else if msg.starts_with("⏺ [error]") {
        // Tool error (will be styled as red in UI)
        let content = msg.strip_prefix("⏺ [error]").unwrap_or(msg).trim();
        app.messages.push(format!("⏺ {}", content));
    } else if msg.starts_with("⏺ ") {
        // In-progress tool (will be styled as orange in UI)
        app.messages.push(msg.to_string());
    } else if msg.contains("\x1b[32m⏺\x1b[0m") || msg.contains("\x1b[31m⏺\x1b[0m") {
        // Legacy ANSI colored format - convert to new format
        let clean_msg = msg
            .replace("\x1b[32m⏺\x1b[0m", "⏺")
            .replace("\x1b[31m⏺\x1b[0m", "⏺");
        app.messages.push(clean_msg);
    } else if msg.starts_with("[tool] ⏺ ") {
        // Legacy format with prefix and indicator
        let content = msg.strip_prefix("[tool] ⏺ ").unwrap_or(msg);
        app.messages.push(format!("⏺ {}", content));
    } else if msg.starts_with("[tool] 🔧") || msg.starts_with("[tool] ") {
        // Other tool formats
        let content = msg
            .strip_prefix("[tool] ")
            .unwrap_or_else(|| msg.strip_prefix("[tool] 🔧").unwrap_or(msg));
        app.messages.push(format!("⏺ {}", content));
    } else if msg.starts_with("Executing tool") || msg.starts_with("Running tool") {
        // Tool execution message
        app.messages.push(format!("⏺ {}", msg));
    } else if msg.starts_with("Using tool") {
        // Tool usage message
        app.messages.push(format!("⏺ {}", msg));
    }

    // Update timestamp and scroll
    app.last_message_time = Instant::now();
    app.auto_scroll_to_bottom();
}

/// Check if a message is a tool result message
fn is_tool_result_message(msg: &str) -> bool {
    msg.starts_with("Tool result:")
}

/// Process tool result messages
fn process_tool_result_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Parse and format the result
    let content = msg.strip_prefix("Tool result:").unwrap_or(msg);
    app.messages
        .push(format!("\x1b[32m⏺\x1b[0m Tool result: {}", content));

    // Update timestamp
    app.last_message_time = Instant::now();
}

/// Check if a message is a success message
fn is_success_message(msg: &str) -> bool {
    msg.starts_with("[success]") || (msg.contains("\x1b[32m⏺\x1b[0m") && !is_tool_message(msg))
}

/// Process success messages
fn process_success_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Parse the content based on format
    let content = if msg.starts_with("[success] ⏺ ") {
        msg.strip_prefix("[success] ⏺ ").unwrap_or(msg)
    } else if msg.starts_with("⏺ ") {
        msg
    } else {
        msg.strip_prefix("[success] ").unwrap_or(msg)
    };

    // Handle multi-line results with tree structure
    if content.contains("\n  ⎿") {
        // Split into header and detail lines
        let parts: Vec<&str> = content.splitn(2, '\n').collect();
        let header = parts[0];

        // Add the header
        app.messages.push(format!("\x1b[32m⏺\x1b[0m {}", header));
        app.last_message_time = Instant::now();

        // Process detail lines if present
        if parts.len() > 1 {
            let lines = parts[1].lines().take(10); // Limit to 10 lines

            for line in lines {
                app.messages.push(line.to_string());
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            // Add indicator if more lines were truncated
            let total_lines = parts[1].lines().count();
            if total_lines > 10 {
                app.messages
                    .push(format!("  ... [{} more lines]", total_lines - 10));
            }
        }
    } else {
        // Simple single-line result
        if content.starts_with("⏺ ") {
            app.messages.push(content.to_string());
        } else {
            app.messages.push(format!("\x1b[32m⏺\x1b[0m {}", content));
        }

        app.last_message_time = Instant::now();
    }

    app.auto_scroll_to_bottom();
}

/// Check if a message is a tool message
fn is_tool_message(msg: &str) -> bool {
    msg.starts_with("[tool]") ||
    msg.contains("\x1b[32m⏺\x1b[0m") || // Legacy green circle for tools
    msg.contains("\x1b[31m⏺\x1b[0m") || // Legacy red circle for errors
    msg.starts_with("⏺ [completed]") || // Completed tool
    msg.starts_with("⏺ [error]") || // Error tool
    (msg.starts_with("⏺ ") && 
     (msg.contains("LS(") || 
      msg.contains("View(") || 
      msg.contains("Glob") || 
      msg.contains("Grep") || 
      msg.contains("Edit") || 
      msg.contains("Replace") || 
      msg.contains("Bash")))
}

/// Check if a message is a thinking message
fn is_thinking_message(msg: &str) -> bool {
    msg.starts_with("Thinking") || msg.contains("[thinking]")
}

/// Process thinking messages
fn process_thinking_message(app: &mut App, msg: &str) {
    // Skip if intermediate steps are disabled
    if !app.show_intermediate_steps {
        return;
    }

    // Skip "analyzing" messages to keep UI clean
    if msg.contains("analyzing") || msg.contains("Analyzing") {
        return;
    }

    // Extract the message content without the [thinking] prefix if present
    let content = msg.strip_prefix("[thinking] ").unwrap_or(msg);

    // Add the thinking message with indicator
    app.messages.push(format!("⚪ {}", content));
}

// Note: Agent-specific methods are now handled in the main message processing logic
// through the MessageType enum, making this function unnecessary
