#![allow(clippy::needless_borrow)]

use crate::app::agent::AgentManager;
use crate::app::commands::CommandHandler;
use crate::app::models::ModelManager;
use crate::app::permissions::PermissionHandler;
use crate::app::state::{App, AppState};
use crate::app::utils::Scrollable;
use crate::ui::draw::ui;
use crate::ui::guards::TerminalGuard;
use crate::ui::messages::{initialize_setup_messages, process_message};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::mpsc, time::Duration};

/// Main application run loop
pub fn run_app() -> Result<()> {
    // Initialize terminal
    let _guard = TerminalGuard::new()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Initialize application state
    let mut app = App::new();

    // Set up welcome messages
    initialize_setup_messages(&mut app);
    app.messages
        .push("DEBUG: Application started. Press Enter to begin setup.".into());

    // Create channel for events
    let (tx, rx) = mpsc::channel::<String>();

    // Initial UI draw
    terminal.draw(|f| ui(f, &app))?;

    // Main event loop
    while app.state != AppState::Error("quit".into()) {
        // Always redraw if download is active, agent is processing, or we're waiting for permission
        if app.download_active || app.agent_progress_rx.is_some() || app.permission_required {
            terminal.draw(|f| ui(f, &app))?;
        }

        // Process messages from various sources
        process_channel_messages(&mut app, &rx, &mut terminal)?;
        process_agent_messages(&mut app, &mut terminal)?;
        process_auto_scroll(&mut app, &mut terminal)?;

        // Check for command mode before handling events (replacing the check in draw.rs)
        if let AppState::Chat = app.state {
            if app.input.starts_with('/') {
                app.check_command_mode();
            }
        }

        // Process user input
        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm::event::read()? {
                // Pass both the key code and the modifiers to the process_key_event function
                process_key_event(&mut app, key.code, key.modifiers, &tx, &mut terminal)?;
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    Ok(())
}

/// Process messages from the message channel
fn process_channel_messages(
    app: &mut App,
    rx: &mpsc::Receiver<String>,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    while let Ok(msg) = rx.try_recv() {
        if app.debug_messages {
            app.messages
                .push(format!("DEBUG: Received message: {}", msg));
        }
        process_message(app, &msg)?;
        terminal.draw(|f| ui(f, app))?;
    }
    Ok(())
}

/// Process messages from the agent progress channel
fn process_agent_messages(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    if let Some(ref agent_rx) = app.agent_progress_rx {
        // Process one message at a time for better visual effect
        if let Ok(msg) = agent_rx.try_recv() {
            // Add debug message if debug is enabled
            if app.debug_messages {
                app.messages
                    .push(format!("DEBUG: Received agent message: {}", msg));
            }

            // Handle ANSI escape sequences by stripping them for storage but preserving their meaning
            let processed_msg = if msg.contains("\x1b[") {
                // Store a version without the ANSI codes for message matching but preserve the styling
                // in a way that the UI can process
                let clean_msg = msg.replace("\x1b[32m", "").replace("\x1b[0m", "");
                if app.debug_messages {
                    app.messages.push(format!("[ansi_styled] {}", clean_msg));
                }

                // Return the message for further processing
                clean_msg
            } else {
                msg
            };

            // Process the message and immediately draw to screen
            process_message(app, &processed_msg)?;

            // Force auto-scroll to keep focus on the latest message
            app.auto_scroll_to_bottom();

            // Immediately redraw after each message for real-time effect
            terminal.draw(|f| ui(f, &app))?;

            // Small delay to create visual animation effect (smaller delay for more fluid updates)
            if !app.permission_required {
                // Don't delay if waiting for permission
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Ok(())
}

/// Process auto-scroll markers in messages
fn process_auto_scroll(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    // Check if we need to auto-scroll after processing messages
    let needs_scroll = app.messages.iter().any(|m| m == "_AUTO_SCROLL_");
    if needs_scroll {
        // Remove the auto-scroll markers
        app.messages.retain(|m| m != "_AUTO_SCROLL_");

        // Actually scroll to bottom
        app.auto_scroll_to_bottom();

        // Redraw with the new scroll position - show updates immediately
        terminal.draw(|f| ui(f, &app))?;
    }
    Ok(())
}

/// Handle Left arrow key for cursor movement
fn handle_left_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Chat | AppState::ApiKeyInput => {
            // Move cursor left if not at the beginning
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
                terminal.draw(|f| ui(f, &app))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle Right arrow key for cursor movement
fn handle_right_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Chat | AppState::ApiKeyInput => {
            // Move cursor right if not at the end
            if app.cursor_position < app.input.len() {
                app.cursor_position += 1;
                terminal.draw(|f| ui(f, &app))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Process keyboard events
fn process_key_event(
    app: &mut App,
    key: KeyCode,
    modifiers: KeyModifiers,
    tx: &mpsc::Sender<String>,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    // Handle permission response first if permission is required
    if app.permission_required {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Grant permission
                app.handle_permission_response(true);
                app.permission_required = false;
                terminal.draw(|f| ui(f, &app))?;
                return Ok(());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Deny permission
                app.handle_permission_response(false);
                app.permission_required = false;
                terminal.draw(|f| ui(f, &app))?;
                return Ok(());
            }
            KeyCode::Esc => {
                // Cancel permission dialog (treat as deny)
                app.handle_permission_response(false);
                app.permission_required = false;
                terminal.draw(|f| ui(f, &app))?;
                return Ok(());
            }
            _ => return Ok(()), // Ignore other keys while permission dialog is active
        }
    }

    // Normal key handling if no permission dialog
    match key {
        KeyCode::Esc => {
            if app.debug_messages {
                app.messages.push("DEBUG: Esc pressed, exiting".into());
            }
            app.state = AppState::Error("quit".into());
        }
        // Enter handling with special case for backslash-newline
        KeyCode::Enter => {
            // Check if the input ends with backslash - treat as newline request
            if app.state == AppState::Chat && app.input.ends_with('\\') {
                // Check if cursor is at the end before making changes
                let cursor_at_end = app.cursor_position == app.input.len();

                // Store cursor position before changes
                let cursor_pos = app.cursor_position;

                // Remove the backslash
                app.input.pop();

                // If cursor was at the end, keep it at the end after changes
                if cursor_at_end {
                    // Add a newline character
                    app.input.push('\n');
                    // Move cursor to end of input
                    app.cursor_position = app.input.len();
                } else if cursor_pos == app.input.len() + 1 {
                    // If cursor was at the position of the removed backslash
                    // Add a newline character
                    app.input.push('\n');
                    // Keep cursor at same position (now after newline)
                    app.cursor_position = app.input.len();
                } else {
                    // If cursor was somewhere else, just add the newline
                    // and adjust cursor position if needed
                    app.input.push('\n');

                    // If cursor was after the removed backslash, adjust it
                    // by moving it back one position
                    if cursor_pos > app.input.len() - 1 {
                        app.cursor_position = cursor_pos - 1;
                    }
                }

                // Force immediate redraw to update input box size and cursor position
                terminal.draw(|f| ui(f, &app))?;
            } else {
                // Regular Enter handling
                handle_enter_key(app, tx, terminal)?;
            }
        }
        KeyCode::Down => handle_down_key(app, terminal)?,
        KeyCode::Tab => handle_tab_key(app, terminal)?,
        KeyCode::Up => handle_up_key(app, terminal)?,
        KeyCode::Left => handle_left_key(app, terminal)?,
        KeyCode::Right => handle_right_key(app, terminal)?,
        KeyCode::BackTab => handle_backtab_key(app, terminal)?,
        KeyCode::Char(c) => handle_char_key(app, c, modifiers, terminal)?,
        KeyCode::Backspace => handle_backspace_key(app, terminal)?,
        KeyCode::PageUp => handle_page_up_key(app, terminal)?,
        KeyCode::PageDown => handle_page_down_key(app, terminal)?,
        KeyCode::Home => handle_home_key(app, terminal)?,
        KeyCode::End => handle_end_key(app, terminal)?,
        _ => {}
    }

    Ok(())
}

/// Handle Enter key in different application states
fn handle_enter_key(
    app: &mut App,
    tx: &mpsc::Sender<String>,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    if app.debug_messages {
        app.messages.push("DEBUG: Enter key pressed".into());
    }

    match app.state {
        AppState::Setup => {
            app.messages.push("DEBUG: Starting model setup...".into());
            terminal.draw(|f| ui(f, &app))?;

            if let Err(e) = app.setup_models(tx.clone()) {
                app.messages.push(format!("ERROR: Setup failed: {}", e));
            }
            terminal.draw(|f| ui(f, &app))?;
        }
        AppState::ApiKeyInput => {
            let api_key = std::mem::take(&mut app.input);
            if !api_key.is_empty() {
                app.messages
                    .push("DEBUG: API key entered, continuing setup...".into());

                // Set the API key and return to setup state
                app.api_key = Some(api_key);
                app.state = AppState::Setup;

                // Continue with model setup using the provided API key
                if let Err(e) = app.setup_models(tx.clone()) {
                    app.messages.push(format!("ERROR: Setup failed: {}", e));
                }
                terminal.draw(|f| ui(f, &app))?;
            } else {
                app.messages
                    .push("API key cannot be empty. Please enter your Anthropic API key...".into());
            }
        }
        AppState::Chat => {
            // First check if we're in command mode
            if app.command_mode {
                // Try to execute the command
                let cmd_executed = app.execute_command();

                // Clear the input field after executing the command
                app.input.clear();
                app.command_mode = false;
                app.show_command_menu = false;

                // Skip model querying if we executed a command
                if cmd_executed {
                    // Need to redraw to clear command menu
                    terminal.draw(|f| ui(f, &app))?;
                    return Ok(());
                }
            }

            let input = std::mem::take(&mut app.input);
            // Reset cursor position since input is cleared
            app.cursor_position = 0;

            if !input.is_empty() {
                // No debug output needed here

                // Add user message with preserved newlines
                app.messages.push(format!("> {}", input));

                // Show a "thinking" message - this will soon be replaced with real-time tool execution
                app.messages.push("[thinking] ⚪ Analyzing query...".into());
                // Force immediate redraw to show thinking state
                app.auto_scroll_to_bottom();
                terminal.draw(|f| ui(f, &app))?;

                // Update the last query time
                app.last_query_time = std::time::Instant::now();

                // Query the model
                match app.query_model(&input) {
                    Ok(response_string) => {
                        // Remove the thinking message (both old and new formats)
                        if let Some(last) = app.messages.last() {
                            if last == "Thinking..." || last.starts_with("[thinking]") {
                                app.messages.pop();
                            }
                        }

                        // Add a clear final answer marker that separates tool results from final response
                        // Only add this marker if we haven't already shown tools during execution
                        let has_tool_markers = app.messages.iter().any(|m| {
                            m.contains("[tool]")
                                || m.contains("⏺ Tool")
                                || m.contains("Executing tool")
                        });
                        if !has_tool_markers {
                            app.messages.push("Final response:".to_string());
                        }

                        // Process and format the response for better display
                        format_and_display_response(app, &response_string);

                        // Force scrolling to the bottom to show the new response
                        app.auto_scroll_to_bottom();

                        // Ensure the UI redraws immediately to show the response
                        terminal.draw(|f| ui(f, &app))?;
                    }
                    Err(e) => {
                        // Remove the thinking message (both old and new formats)
                        if let Some(last) = app.messages.last() {
                            if last == "Thinking..." || last.starts_with("[thinking]") {
                                app.messages.pop();
                            }
                        }
                        app.messages.push(format!("Error: {}", e));
                        app.auto_scroll_to_bottom();
                    }
                }

                // Make sure to redraw after getting a response
                terminal.draw(|f| ui(f, &app))?;
            }
        }
        AppState::Error(_) => {
            app.state = AppState::Setup;
            app.error_message = None;
        }
    }
    terminal.draw(|f| ui(f, &app))?;

    Ok(())
}

/// Format and display a model response
fn format_and_display_response(app: &mut App, response: &str) {
    // Split long responses into multiple messages if needed
    let max_line_length = 80; // Reasonable line length for TUI display

    if response.contains('\n') {
        // For multi-line responses (code or structured content)
        // Add an empty line before the response for readability
        app.messages.push("".to_string());

        // Split by line to preserve formatting
        for line in response.lines() {
            // For very long lines, add wrapping
            if line.len() > max_line_length {
                // Simple wrapping at character boundaries
                // Use integer division that rounds up (equivalent to ceiling division)
                // Skip clippy suggestion as div_ceil might not be available in all Rust versions
                #[allow(clippy::manual_div_ceil)]
                let chunk_count = (line.len() + max_line_length - 1) / max_line_length;
                for i in 0..chunk_count {
                    let start = i * max_line_length;
                    let end = std::cmp::min(start + max_line_length, line.len());
                    if start < line.len() {
                        app.messages.push(line[start..end].to_string());
                    }
                }
            } else {
                app.messages.push(line.to_string());
            }
        }

        // Add another empty line after for readability
        app.messages.push("".to_string());
    } else {
        // For single-line responses, add directly
        app.messages.push(response.to_string());
    }
}

/// Handle Down key in different application states
fn handle_down_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Setup => {
            app.select_next_model();
            app.messages.push("DEBUG: Selected next model".into());
            terminal.draw(|f| ui(f, &app))?;
        }
        AppState::Chat => {
            // Navigate commands in command mode
            if app.show_command_menu {
                app.select_next_command();
                terminal.draw(|f| ui(f, &app))?;
            }
            // When not in command mode and multiline input, move cursor down
            else if app.input.contains('\n') {
                // Find the current line and position within that line
                let mut current_pos = 0;
                let mut current_line_start = 0;
                let mut next_line_start = 0;
                let mut found_next_line = false;

                // Analyze the input to find line boundaries
                for (i, c) in app.input.chars().enumerate() {
                    if i == app.cursor_position {
                        // Mark the position on the current line
                        current_pos = i - current_line_start;
                    }

                    if c == '\n' {
                        if i < app.cursor_position {
                            // This newline is before the cursor, update current line
                            current_line_start = i + 1;
                        } else if !found_next_line {
                            // This is the end of the current line (start of next line)
                            next_line_start = i + 1;
                            found_next_line = true;
                        }
                    }
                }

                // Only move down if there's a line below
                if found_next_line && next_line_start < app.input.len() {
                    // Find the position on the next line (same column if possible)
                    let next_line_end = app.input[next_line_start..]
                        .find('\n')
                        .map(|pos| next_line_start + pos)
                        .unwrap_or(app.input.len());

                    // Calculate the new cursor position
                    let next_line_length = next_line_end - next_line_start;
                    let new_pos = next_line_start + current_pos.min(next_line_length);

                    // Update cursor position
                    app.cursor_position = new_pos;
                    terminal.draw(|f| ui(f, &app))?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle Tab key in different application states
fn handle_tab_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Setup => {
            app.select_next_model();
            app.messages.push("DEBUG: Selected next model".into());
            terminal.draw(|f| ui(f, &app))?;
        }
        AppState::Chat => {
            // Auto-complete command if in command mode
            if app.show_command_menu {
                let filtered = app.filtered_commands();
                if !filtered.is_empty() && app.selected_command < filtered.len() {
                    // Auto-complete with selected command
                    app.input = filtered[app.selected_command].name.clone();
                    app.show_command_menu = true;
                    app.command_mode = true;
                }
                terminal.draw(|f| ui(f, &app))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle Up key in different application states
fn handle_up_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Setup => {
            app.select_prev_model();
            app.messages.push("DEBUG: Selected previous model".into());
            terminal.draw(|f| ui(f, &app))?;
        }
        AppState::Chat => {
            // Navigate commands in command mode
            if app.show_command_menu {
                app.select_prev_command();
                terminal.draw(|f| ui(f, &app))?;
            }
            // When not in command mode and multiline input, move cursor up
            else if app.input.contains('\n') {
                // Find the current line and position within that line
                let mut current_pos = 0;
                let mut current_line_start = 0;
                let mut prev_line_start = 0;
                let mut prev_line_end = 0;
                let mut on_first_line = true;

                // Analyze the input to find line boundaries
                for (i, c) in app.input.chars().enumerate() {
                    if c == '\n' && i < app.cursor_position {
                        // This newline is before the cursor
                        prev_line_start = current_line_start;
                        prev_line_end = i;
                        current_line_start = i + 1;
                        on_first_line = false;
                    }

                    if i == app.cursor_position {
                        // Mark the position on the current line
                        current_pos = i - current_line_start;
                    }
                }

                // Only move up if not on the first line
                if !on_first_line {
                    // Calculate the new cursor position
                    let prev_line_length = prev_line_end - prev_line_start;
                    let new_pos = prev_line_start + current_pos.min(prev_line_length);

                    // Update cursor position
                    app.cursor_position = new_pos;
                    terminal.draw(|f| ui(f, &app))?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle BackTab key in different application states
fn handle_backtab_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    if let AppState::Setup = app.state {
        app.select_prev_model();
        app.messages.push("DEBUG: Selected previous model".into());
        terminal.draw(|f| ui(f, &app))?;
    }
    Ok(())
}

/// Handle character key in different application states
fn handle_char_key(
    app: &mut App,
    c: char,
    _: KeyModifiers, // Unused parameter
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Chat | AppState::ApiKeyInput => {
            // Newlines are handled in the Enter key handler

            // Insert character at cursor position instead of appending
            app.input.insert(app.cursor_position, c);

            // Move cursor forward (after the just-inserted character)
            app.cursor_position += 1;

            // Make sure cursor is always within bounds
            app.cursor_position = app.cursor_position.min(app.input.len());

            // Force immediate redraw to ensure cursor remains visible
            terminal.draw(|f| ui(f, &app))?;

            // Check if we're entering command mode with the / character
            if app.state == AppState::Chat && c == '/' && app.input.len() == 1 {
                app.command_mode = true;
                app.show_command_menu = true;
                app.selected_command = 0;
                // Hide detailed shortcuts when typing /
                app.show_detailed_shortcuts = false;
            } else if app.state == AppState::Chat && c == '?' && app.input.len() == 1 {
                // Toggle detailed shortcuts display and clear the '?' from input
                app.show_detailed_shortcuts = !app.show_detailed_shortcuts;
                app.input.clear();
                app.cursor_position = 0; // Reset cursor position
            } else if app.command_mode {
                // Update command mode state
                app.check_command_mode();
            } else {
                // Hide detailed shortcuts when typing anything else
                app.show_detailed_shortcuts = false;
            }

            terminal.draw(|f| ui(f, &app))?;
        }
        _ => {}
    }
    Ok(())
}

/// Handle backspace key in different application states
fn handle_backspace_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Chat | AppState::ApiKeyInput => {
            // Only delete a character if the cursor is not at the beginning of the string
            if app.cursor_position > 0 {
                // Remove the character before the cursor
                app.input.remove(app.cursor_position - 1);
                // Move the cursor back one position
                app.cursor_position -= 1;
            }

            // Check if we've exited command mode
            if app.state == AppState::Chat {
                app.check_command_mode();
            }

            terminal.draw(|f| ui(f, &app))?;
        }
        _ => {}
    }
    Ok(())
}

/// Handle PageUp key for scrolling
fn handle_page_up_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    if let AppState::Chat = app.state {
        app.scroll_up(5); // Scroll up 5 lines
        terminal.draw(|f| ui(f, &app))?;
    }
    Ok(())
}

/// Handle PageDown key for scrolling
fn handle_page_down_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    if let AppState::Chat = app.state {
        app.scroll_down(5); // Scroll down 5 lines
        terminal.draw(|f| ui(f, &app))?;
    }
    Ok(())
}

/// Handle Home key for scrolling to top and moving cursor to start of input
fn handle_home_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Chat => {
            if app.input.is_empty() {
                // If no input, use Home to scroll the message window to top
                app.scroll_position = 0; // Scroll to top
            } else if app.input.contains('\n') {
                // In multiline mode, go to start of current line
                let mut current_line_start = 0;

                // Find the start of the current line
                for (i, c) in app.input.chars().enumerate() {
                    if c == '\n' && i < app.cursor_position {
                        current_line_start = i + 1;
                    }
                    if i >= app.cursor_position {
                        break;
                    }
                }

                // Move to start of current line
                app.cursor_position = current_line_start;
            } else {
                // In single line mode, go to start of input
                app.cursor_position = 0;
            }
            terminal.draw(|f| ui(f, &app))?;
        }
        AppState::ApiKeyInput => {
            // Move cursor to start of input
            app.cursor_position = 0;
            terminal.draw(|f| ui(f, &app))?;
        }
        _ => {}
    }
    Ok(())
}

/// Handle End key for scrolling to bottom and moving cursor to end of input
fn handle_end_key(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match app.state {
        AppState::Chat => {
            if app.input.is_empty() {
                // If no input, use End to scroll the message window to bottom
                app.auto_scroll_to_bottom(); // Scroll to bottom
            } else if app.input.contains('\n') {
                // In multiline mode, go to end of current line
                let mut current_line_end = app.input.len();

                // Find the end of the current line
                for (i, c) in app.input.chars().enumerate() {
                    if i <= app.cursor_position {
                        continue;
                    }
                    if c == '\n' {
                        current_line_end = i;
                        break;
                    }
                }

                // Move to end of current line
                app.cursor_position = current_line_end;
            } else {
                // In single line mode, go to end of input
                app.cursor_position = app.input.len();
            }
            terminal.draw(|f| ui(f, &app))?;
        }
        AppState::ApiKeyInput => {
            // Move cursor to end of input
            app.cursor_position = app.input.len();
            terminal.draw(|f| ui(f, &app))?;
        }
        _ => {}
    }
    Ok(())
}
