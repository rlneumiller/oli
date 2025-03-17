use crate::app::{App, AppState};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{io, time::Duration};
use unicode_width::UnicodeWidthStr;

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen, Show);
        let _ = disable_raw_mode();
    }
}

pub fn run_app() -> Result<()> {
    let _guard = TerminalGuard::new()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut app = App::new();
    // Initialize welcome messages only for the setup screen
    initialize_setup_messages(&mut app);
    app.messages
        .push("DEBUG: Application started. Press Enter to begin setup.".into());

    let (tx, rx) = std::sync::mpsc::channel::<String>();

    terminal.draw(|f| ui(f, &app))?;

    while app.state != AppState::Error("quit".into()) {
        // Always redraw if download is active or agent is processing, to show progress
        if app.download_active || app.agent_progress_rx.is_some() {
            terminal.draw(|f| ui(f, &app))?;
        }

        // Check for messages from download thread
        while let Ok(msg) = rx.try_recv() {
            app.messages
                .push(format!("DEBUG: Received message: {}", msg));
            process_message(&mut app, &msg)?;
            terminal.draw(|f| ui(f, &app))?;
        }

        // Check for messages from agent progress - process one at a time for better animation effect
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
                    app.messages.push(format!("[ansi_styled] {}", clean_msg));

                    // Return the message for further processing
                    clean_msg
                } else {
                    msg
                };

                // Process the message and immediately draw to screen
                process_message(&mut app, &processed_msg)?;
                
                // Force auto-scroll to keep focus on the latest message
                app.auto_scroll_to_bottom();
                
                // Immediately redraw after each message for real-time effect
                terminal.draw(|f| ui(f, &app))?;
                
                // Small delay to create visual animation effect (200ms between updates)
                std::thread::sleep(Duration::from_millis(200));
            }
        }

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

        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        app.messages.push("DEBUG: Esc pressed, exiting".into());
                        break;
                    }
                    KeyCode::Enter => {
                        app.messages.push("DEBUG: Enter key pressed".into());

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
                                    app.messages.push("API key cannot be empty. Please enter your Anthropic API key...".into());
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
                                        continue;
                                    }
                                }

                                let input = std::mem::take(&mut app.input);
                                if !input.is_empty() {
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
                                        Ok(response) => {
                                            // Remove the thinking message (both old and new formats)
                                            if let Some(last) = app.messages.last() {
                                                if last == "Thinking..."
                                                    || last.starts_with("[thinking]")
                                                {
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
                                                        let chunk_count =
                                                            (line.len() + max_line_length - 1)
                                                                / max_line_length;
                                                        for i in 0..chunk_count {
                                                            let start = i * max_line_length;
                                                            let end = std::cmp::min(
                                                                start + max_line_length,
                                                                line.len(),
                                                            );
                                                            if start < line.len() {
                                                                app.messages.push(
                                                                    line[start..end].to_string(),
                                                                );
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
                                                app.messages.push(response);
                                            }

                                            // Force scrolling to the bottom to show the new response
                                            app.auto_scroll_to_bottom();

                                            // Ensure the UI redraws immediately to show the response
                                            terminal.draw(|f| ui(f, &app))?;
                                        }
                                        Err(e) => {
                                            // Remove the thinking message (both old and new formats)
                                            if let Some(last) = app.messages.last() {
                                                if last == "Thinking..."
                                                    || last.starts_with("[thinking]")
                                                {
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
                    }
                    KeyCode::Down => {
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
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Tab => {
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
                                    if !filtered.is_empty() && app.selected_command < filtered.len()
                                    {
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
                    }
                    KeyCode::Up => {
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
                            }
                            _ => {}
                        }
                    }
                    KeyCode::BackTab => {
                        if let AppState::Setup = app.state {
                            app.select_prev_model();
                            app.messages.push("DEBUG: Selected previous model".into());
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    KeyCode::Char(c) => match app.state {
                        AppState::Chat | AppState::ApiKeyInput => {
                            app.input.push(c);

                            // Check if we're entering command mode with the / character
                            if app.state == AppState::Chat && c == '/' && app.input.len() == 1 {
                                app.command_mode = true;
                                app.show_command_menu = true;
                                app.selected_command = 0;
                            } else if app.command_mode {
                                // Update command mode state
                                app.check_command_mode();
                            }

                            terminal.draw(|f| ui(f, &app))?;
                        }
                        _ => {}
                    },
                    KeyCode::Backspace => match app.state {
                        AppState::Chat | AppState::ApiKeyInput => {
                            app.input.pop();

                            // Check if we've exited command mode
                            if app.state == AppState::Chat {
                                app.check_command_mode();
                            }

                            terminal.draw(|f| ui(f, &app))?;
                        }
                        _ => {}
                    },
                    // Handle scrolling in chat mode
                    KeyCode::PageUp => {
                        if let AppState::Chat = app.state {
                            app.scroll_up(5); // Scroll up 5 lines
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    KeyCode::PageDown => {
                        if let AppState::Chat = app.state {
                            app.scroll_down(5); // Scroll down 5 lines
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    // Home key for top
                    KeyCode::Home => {
                        if let AppState::Chat = app.state {
                            app.scroll_position = 0; // Scroll to top
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    // End key for bottom
                    KeyCode::End => {
                        if let AppState::Chat = app.state {
                            app.auto_scroll_to_bottom(); // Scroll to bottom
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    Ok(())
}

fn process_message(app: &mut App, msg: &str) -> Result<()> {
    app.messages
        .push(format!("DEBUG: Processing message: {}", msg));

    if msg.starts_with("progress:") {
        // Make sure download_active is true whenever we receive progress
        app.download_active = true;

        let parts: Vec<&str> = msg.split(':').collect();
        if parts.len() >= 3 {
            if let (Ok(downloaded), Ok(total)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>())
            {
                app.download_progress = Some((downloaded, total));
                // Only log progress occasionally to avoid flooding logs
                if downloaded % (5 * 1024 * 1024) < 100000 {
                    // Log roughly every 5MB
                    app.messages.push(format!(
                        "DEBUG: Download progress: {:.1}MB/{:.1}MB",
                        downloaded as f64 / 1_000_000.0,
                        total as f64 / 1_000_000.0
                    ));
                }
            }
        }
    } else if msg.starts_with("status:") {
        // Status updates for the download process
        let status = msg.replacen("status:", "", 1);
        app.messages.push(format!("Status: {}", status));
    } else if msg.starts_with("download_started:") {
        app.download_active = true;
        let url = msg.replacen("download_started:", "", 1);
        app.messages.push(format!("Starting download from {}", url));
    } else if msg == "download_complete" {
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
    } else if msg == "api_key_needed" {
        // Special case for when we need an API key
        app.messages
            .push("Please enter your Anthropic API key to use Claude 3.7...".into());
    } else if msg == "setup_complete" {
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
    } else if msg == "setup_failed" {
        app.messages
            .push("Setup failed. Check error messages above.".into());
    } else if msg.starts_with("error:") {
        let error_msg = msg.replacen("error:", "", 1);
        app.error_message = Some(error_msg.clone());
        app.state = AppState::Error(error_msg);
    } else if msg.starts_with("retry:") {
        app.messages.push(msg.replacen("retry:", "", 1));
    } else if msg.starts_with("Executing tool") || msg.starts_with("Running tool") {
        // Show the raw tool execution messages to indicate tool usage during query
        app.messages.push(format!("⚙️ {}", msg));
        // Force immediate redraw to show tool usage in real-time
        app.auto_scroll_to_bottom();
    } else if msg.starts_with("Sending request to AI")
        || msg.starts_with("Processing tool results")
        || msg.starts_with("[wait]")
    {
        // Handle AI operation with white circle - standardize all waiting operations
        if msg.starts_with("[wait]") {
            // Already formatted with the white circle from agent executor
            app.messages.push(msg.to_string());
        } else {
            // Legacy format that needs the circle added
            app.messages.push(format!("⚪ {}", msg));
        }
        // Force immediate redraw to show AI thinking in real-time
        // Add timestamp to make this message "active" for a short period
        app.last_message_time = std::time::Instant::now();
        app.auto_scroll_to_bottom();
    } else if msg.starts_with("[tool]") {
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
    } else if msg.starts_with("Tool result:") {
        // Display tool results directly
        let content = msg.strip_prefix("Tool result:").unwrap_or(msg);
        app.messages
            .push(format!("\x1b[32m⏺\x1b[0m Tool result: {}", content));
    } else if msg.starts_with("[success]") {
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
                    std::thread::sleep(Duration::from_millis(50));
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
    } else if msg.starts_with("Using tool") {
        // Show tool usage messages
        app.messages.push(format!("⚙️ {}", msg));
        app.auto_scroll_to_bottom();
    } else if msg.starts_with("Thinking") || msg.contains("analyzing") {
        // Handle AI thinking process messages with white circle
        app.messages.push(format!("⚪ {}", msg));
    } else if msg == "Agent initialized successfully" {
        app.messages
            .push("⏺ Agent initialized and ready to use!".into());
    } else if msg.starts_with("Failed to initialize agent") {
        app.messages.push(format!("[error] ❌ {}", msg));
        app.use_agent = false;
    } else if msg.contains("completed successfully") || msg.contains("done") {
        app.messages.push(format!("⏺ {}", msg));
    }

    Ok(())
}

// Initialize messages specifically for the setup screen
fn initialize_setup_messages(app: &mut App) {
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

fn ui(f: &mut Frame, app: &App) {
    match app.state {
        AppState::Setup => draw_setup(f, app),
        AppState::ApiKeyInput => draw_api_key_input(f, app),
        AppState::Chat => draw_chat(f, app),
        AppState::Error(ref error_msg) => draw_error(f, app, error_msg),
    }

    // Check for command mode on each UI update
    if let AppState::Chat = app.state {
        if app.input.starts_with('/') {
            // This will be handled in the draw_chat function
            let app_mut = app as *const App as *mut App;
            unsafe {
                (*app_mut).check_command_mode();
            }
        }
    }
}

fn draw_setup(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("OLI Setup Assistant")
        .style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Model list
    let models: Vec<ListItem> = app
        .available_models
        .iter()
        .enumerate()
        .map(|(i, model)| {
            let content = format!(
                "{} ({:.2}GB) - {}",
                model.name, model.size_gb, model.description
            );
            if i == app.selected_model {
                ListItem::new(format!("→ {}", content)).style(Style::default().fg(Color::Yellow))
            } else {
                ListItem::new(format!("  {}", content))
            }
        })
        .collect();

    let models_list = List::new(models)
        .block(Block::default().borders(Borders::ALL).title("Models"))
        .highlight_style(Style::default().fg(Color::Yellow));
    f.render_widget(models_list, chunks[1]);

    // Progress
    let progress_text = if app.download_active {
        app.download_progress.map_or_else(
            || "Preparing download...".into(),
            |(d, t)| {
                let percent = if t > 0 {
                    (d as f64 / t as f64) * 100.0
                } else {
                    0.0
                };

                // Create a visual progress bar
                let bar_width = 50; // Number of characters for the progress bar
                let filled = (percent / 100.0 * bar_width as f64) as usize;
                let empty = bar_width - filled;
                let progress_bar = format!(
                    "[{}{}] {:.1}%",
                    "=".repeat(filled),
                    " ".repeat(empty),
                    percent
                );

                format!(
                    "{}\nDownloading {}: {:.2}MB of {:.2}MB",
                    progress_bar,
                    app.current_model().file_name,
                    d as f64 / 1_000_000.0,
                    t as f64 / 1_000_000.0
                )
            },
        )
    } else {
        "Press Enter to begin setup".into()
    };

    let progress_bar = Paragraph::new(progress_text)
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .style(Style::default().fg(Color::Green));
    f.render_widget(progress_bar, chunks[2]);
}

fn draw_chat(f: &mut Frame, app: &App) {
    // Use three chunks - header, message history, and input (with command menu if active)
    let input_height = if app.show_command_menu {
        // Increase the input area height to make room for the command menu
        let cmd_count = app.filtered_commands().len();
        // Limit to 5 commands at a time, with at least 3 lines for input
        3 + cmd_count.min(5)
    } else {
        3 // Default input height
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),                   // Status bar
            Constraint::Min(3),                      // Chat history (expandable)
            Constraint::Length(input_height as u16), // Input area (with variable height for command menu)
        ])
        .split(f.area());

    // Status bar showing model info and scroll position
    let model_name = app.current_model().name.clone();
    let scroll_info = format!(
        "Scroll: {}/{} (PageUp/PageDown to scroll)",
        app.scroll_position,
        app.messages.len().saturating_sub(10)
    );

    // Add agent indicator if agent is available
    let agent_indicator = if app.use_agent && app.agent.is_some() {
        Span::styled(
            " 🤖 Agent ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " 🖥️ Local ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        )
    };

    let status_bar = Line::from(vec![
        Span::styled(
            format!(" Model: {} ", model_name),
            Style::default().fg(Color::LightCyan).bg(Color::DarkGray),
        ),
        Span::raw(" "),
        agent_indicator,
        Span::raw(" | "),
        Span::styled(scroll_info, Style::default().fg(Color::DarkGray)),
        Span::raw(" | "),
        Span::styled(
            " PgUp/PgDn: Scroll  Esc: Quit  Type / for commands ",
            Style::default().fg(Color::Black).bg(Color::LightBlue),
        ),
    ]);

    let status_bar_widget = Paragraph::new(status_bar).style(Style::default());
    f.render_widget(status_bar_widget, chunks[0]);

    // Filter and style messages
    // First, clean up any invisible markers
    let display_messages: Vec<&String> = app
        .messages
        .iter()
        .filter(|msg| *msg != "_AUTO_SCROLL_")
        .collect();

    // Calculate if we should show animation effects (blinking) for new messages
    // Messages added in the last second get a highlight effect
    let animation_active = app.last_message_time.elapsed() < Duration::from_millis(1000);
    // Blink rate - make it blink about twice per second for newly added messages
    let highlight_on = animation_active && 
        (std::time::Instant::now().elapsed().as_millis() % 500) < 300; // blink pattern

    // Then apply scrolling and create styled Lines
    let visible_messages: Vec<Line> = display_messages
        .iter()
        .enumerate()
        // Apply scrolling - show messages based on scroll position
        .filter(|(idx, _)| {
            // Only show messages at or after the scroll position
            *idx >= app.scroll_position &&
            // Only show messages that would fit in the visible area
            *idx < app.scroll_position + chunks[1].height as usize
        })
        .map(|(idx, &m)| {
            // Check if this is the last message and should be highlighted
            let is_newest_msg = idx == display_messages.len() - 1;
            if m.starts_with("DEBUG:") {
                // Only show debug messages in debug mode
                if app.debug_messages {
                    Line::from(vec![Span::styled(
                        m.as_str(),
                        Style::default().fg(Color::Yellow),
                    )])
                } else {
                    Line::from("")
                }
            } else if let Some(stripped) = m.strip_prefix("> ") {
                // User messages - cyan
                Line::from(vec![
                    Span::styled(
                        "YOU: ",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(stripped, Style::default().fg(Color::Cyan)),
                ])
            } else if m.starts_with("Error:") || m.starts_with("ERROR:") {
                // Error messages - red
                Line::from(vec![Span::styled(
                    m.as_str(),
                    Style::default().fg(Color::Red),
                )])
            } else if m.starts_with("Status:") {
                // Status messages - blue
                Line::from(vec![Span::styled(
                    m.as_str(),
                    Style::default().fg(Color::Blue),
                )])
            } else if m.starts_with("★") {
                // Title/welcome messages - light cyan with bold
                Line::from(vec![Span::styled(
                    m.as_str(),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )])
            } else if m == "Thinking..." {
                // Legacy thinking message
                Line::from(vec![
                    Span::styled("⏺ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "Thinking...",
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])
            } else if m.starts_with("[thinking]") {
                // AI Thinking/reasoning message
                let thinking_content = m.strip_prefix("[thinking] ").unwrap_or(m);
                
                // Add pulsing animation effect to make it more noticeable
                let style = if is_newest_msg {
                    // Always highlight thinking messages when they're new
                    // This creates a pulsing effect by alternating between normal and bright
                    if highlight_on {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::ITALIC)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::ITALIC)
                    }
                } else {
                    // Normal style for older messages
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::ITALIC)
                };

                if thinking_content.starts_with("⚪ ") {
                    // New format with white circle - already has the icon
                    Line::from(vec![Span::styled(m, style)])
                } else {
                    // Legacy format without icon - add the circle
                    Line::from(vec![
                        Span::styled(
                            "⏺ ", 
                            if is_newest_msg && highlight_on {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Yellow)
                            }
                        ),
                        Span::styled(thinking_content, style),
                    ])
                }
            } else if m.starts_with("[tool] ") {
                // Tool execution message
                let tool_content = m.strip_prefix("[tool] ").unwrap_or(m);
                
                // Apply animation highlight effect for newest message
                let style = if is_newest_msg && highlight_on {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD)
                };

                if tool_content.starts_with("🟢 ") || tool_content.starts_with("🔧 ") {
                    // New format with emoji icon
                    Line::from(vec![Span::styled(
                        tool_content,
                        style,
                    )])
                } else {
                    // Legacy format with old indicator
                    Line::from(vec![
                        Span::styled(
                            "⏺ ", 
                            if is_newest_msg && highlight_on {
                                Style::default().fg(Color::Blue).bg(Color::LightBlue)
                            } else {
                                Style::default().fg(Color::Blue)
                            }
                        ),
                        Span::styled(
                            tool_content,
                            style,
                        ),
                    ])
                }
            } else if m.starts_with("[success] ") {
                // Success/completion message - probably tool results
                let content = m.strip_prefix("[success] ").unwrap_or(m);

                // Check for green circle in the content
                if content.starts_with("🟢 Tool result:") {
                    let mut lines = Vec::new();

                    // First line with the green circle emoji
                    let tool_msg = content.strip_prefix("🟢 ").unwrap_or(content);
                    lines.push(Line::from(vec![
                        Span::styled("⏺ ", Style::default().fg(Color::Green)), // Smaller circle
                        Span::styled(
                            tool_msg,
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    // Skip the "Tool result:" prefix and add the actual output indented
                    if let Some(tool_output) = tool_msg.strip_prefix("Tool result:") {
                        // Handle multiline results
                        for line in tool_output.trim().lines() {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default().fg(Color::Green)),
                                Span::styled(line, Style::default().fg(Color::Green)),
                            ]));
                        }
                    }

                    // Return the first line, the rest will be added to the text in the calling context
                    lines.first().cloned().unwrap_or_default()
                } else if content.starts_with("Tool result:") {
                    // Legacy format
                    let mut lines = Vec::new();

                    // First line gets the icon
                    lines.push(Line::from(vec![
                        Span::styled("⏺ ", Style::default().fg(Color::Green)),
                        Span::styled(
                            "Tool result:",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    // Skip the "Tool result:" prefix and add the actual output indented
                    if let Some(tool_output) = content.strip_prefix("Tool result:") {
                        // Handle multiline results
                        for line in tool_output.trim().lines() {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default().fg(Color::Green)),
                                Span::styled(line, Style::default().fg(Color::Green)),
                            ]));
                        }
                    }

                    // Return the first line, the rest will be added to the text in the calling context
                    lines.first().cloned().unwrap_or_default()
                } else if content.starts_with("🟢 ") {
                    // Regular success message with green circle - make it smaller
                    let msg = content.strip_prefix("🟢 ").unwrap_or(content);
                    Line::from(vec![
                        Span::styled("⏺ ", Style::default().fg(Color::Green)),
                        Span::styled(msg, Style::default().fg(Color::Green)),
                    ])
                } else {
                    // Legacy format for regular success message
                    Line::from(vec![
                        Span::styled("⏺ ", Style::default().fg(Color::Green)),
                        Span::styled(content, Style::default().fg(Color::Green)),
                    ])
                }
            } else if m.starts_with("[wait]") {
                // Progress/wait message with white circle
                let wait_content = m.strip_prefix("[wait] ").unwrap_or(m);

                if wait_content.starts_with("⚪ ") || m.contains("⚪") {
                    // New format already has the white circle emoji
                    // Keep the full message since it already has the icon
                    Line::from(vec![Span::styled(m, Style::default().fg(Color::Yellow))])
                } else {
                    // Legacy format needs an icon
                    Line::from(vec![
                        Span::styled("⏺ ", Style::default().fg(Color::LightYellow)),
                        Span::styled(wait_content, Style::default().fg(Color::Yellow)),
                    ])
                }
            } else if m.starts_with("[error] ") {
                // Error/failure message
                let error_content = m.strip_prefix("[error] ").unwrap_or(m);

                if error_content.starts_with("❌ ") {
                    // New format with X mark emoji
                    Line::from(vec![Span::styled(
                        error_content,
                        Style::default().fg(Color::Red),
                    )])
                } else {
                    // Legacy format
                    Line::from(vec![
                        Span::styled("⏺ ", Style::default().fg(Color::Red)),
                        Span::styled(error_content, Style::default().fg(Color::Red)),
                    ])
                }
            } else {
                // Model responses - with styling for code blocks
                if m == "Final response:" {
                    // Special formatting for the final response marker
                    Line::from(vec![
                        Span::styled("✨ ", Style::default().fg(Color::Green)),
                        Span::styled(
                            "Final Response:",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else if m.trim().is_empty() {
                    Line::from("")
                } else if !m.starts_with("> ")
                    && !m.starts_with("DEBUG:")
                    && app.messages.contains(&format!("> {}", app.input))
                {
                    // This is likely a model response
                    // Split long lines to ensure they stay within the UI width
                    let max_width = chunks[1].width as usize - 10; // Subtract some padding

                    if m.contains('\n') || m.len() > max_width {
                        // For multi-line or very long responses, return a formatted header line
                        // The actual content will be handled in the paragraph rendering

                        // Create a Final answer header with white circle (for response following tools)
                        let header = Span::styled("⏺ ", Style::default().fg(Color::White));

                        let final_answer = Span::styled(
                            "Final answer:",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        );

                        // Just return the header with the first line of content
                        // The rest will be properly wrapped by the paragraph widget
                        if m.contains('\n') {
                            let first_line = m.lines().next().unwrap_or("");
                            Line::from(vec![
                                header,
                                final_answer,
                                Span::raw(" "),
                                Span::raw(first_line),
                            ])
                        } else {
                            // For long single-line responses, let the widget handle wrapping
                            Line::from(vec![header, final_answer, Span::raw(" "), Span::raw(m)])
                        }
                    } else {
                        // For short responses, show in a single line with Final answer header
                        Line::from(vec![
                            Span::styled("⏺ ", Style::default().fg(Color::White)),
                            Span::styled(
                                "Final answer:",
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(" "),
                            Span::raw(m),
                        ])
                    }
                } else {
                    // Regular text
                    Line::from(m.as_str())
                }
            }
        })
        .collect();

    // Create a scrollable paragraph for the messages
    let has_more_above = app.scroll_position > 0;
    let has_more_below = app.scroll_position + (chunks[1].height as usize) < app.messages.len();

    // Create title with scroll indicators
    let title = if has_more_above && has_more_below {
        Line::from(vec![
            Span::raw("OLI Assistant "),
            Span::styled("▲ more above ", Style::default().fg(Color::DarkGray)),
            Span::styled("▼ more below", Style::default().fg(Color::DarkGray)),
        ])
    } else if has_more_above {
        Line::from(vec![
            Span::raw("OLI Assistant "),
            Span::styled("▲ more above", Style::default().fg(Color::DarkGray)),
        ])
    } else if has_more_below {
        Line::from(vec![
            Span::raw("OLI Assistant "),
            Span::styled("▼ more below", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from("OLI Assistant")
    };

    let message_block = Block::default().borders(Borders::ALL).title(title);

    // Ensure proper scrollable behavior with fixed height
    // Use ratatui's scrolling paragraphs for smoother scrolling behavior
    let messages_window = Paragraph::new(Text::from(visible_messages))
        .block(message_block)
        .wrap(Wrap { trim: false }) // Set trim to false to preserve message formatting
        .scroll((0, 0)); // Explicit scrolling control to prevent auto-scrolling issues
    f.render_widget(messages_window, chunks[1]);

    // Split the input area if command menu is visible
    if app.show_command_menu {
        // Split the input area into the input box and command menu
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                                           // Input box
                Constraint::Length(app.filtered_commands().len().min(5) as u16), // Command menu (max 5 items)
            ])
            .split(chunks[2]);

        // Input box with hint text
        let input_text = if app.input.is_empty() {
            Span::styled(
                "Type / to show commands or ask a question...",
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw(app.input.as_str())
        };

        let input_window = Paragraph::new(input_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Input (Type / for commands)")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(input_window, input_chunks[0]);

        // Commands menu as a list
        let filtered_commands = app.filtered_commands();
        // Ensure selected command is in bounds
        let valid_selected = if filtered_commands.is_empty() {
            0
        } else {
            app.selected_command.min(filtered_commands.len() - 1)
        };

        let command_items: Vec<ListItem> = filtered_commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                if i == valid_selected {
                    // Highlight the selected command with an arrow indicator
                    ListItem::new(format!("▶ {} - {}", cmd.name, cmd.description))
                        .style(Style::default().fg(Color::Black).bg(Color::LightCyan))
                } else {
                    // Non-selected commands with proper spacing
                    ListItem::new(format!("  {} - {}", cmd.name, cmd.description))
                        .style(Style::default().fg(Color::Gray))
                }
            })
            .collect();

        // Create the list with a subtle style
        let commands_list = List::new(command_items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::Gray)) // Default text color
            .highlight_style(Style::default().fg(Color::Black).bg(Color::LightCyan)); // Selected item style
        f.render_widget(commands_list, input_chunks[1]);

        // Set cursor position at end of input
        if !app.input.is_empty() {
            f.set_cursor_position((
                input_chunks[0].x + app.input.width() as u16 + 1,
                input_chunks[0].y + 1,
            ));
        }
    } else {
        // Regular input box without command menu
        // Input box with hint text
        let input_text = if app.input.is_empty() {
            Span::styled(
                "Type / to show commands or ask a question...",
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw(app.input.as_str())
        };

        let input_window = Paragraph::new(input_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Input (Type / for commands)")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(input_window, chunks[2]);

        // Only show cursor if there is input
        if !app.input.is_empty() {
            // Set cursor position at end of input
            f.set_cursor_position((chunks[2].x + app.input.width() as u16 + 1, chunks[2].y + 1));
        }
    }
}

fn draw_api_key_input(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Determine title and info based on selected model
    let (title_text, message_items) = match app.current_model().name.as_str() {
        "GPT-4o" => (
            "OpenAI API Key Setup",
            vec![
                ListItem::new("To use GPT-4o, you need to provide your OpenAI API key."),
                ListItem::new("You can get an API key from https://platform.openai.com/api-keys"),
                ListItem::new(""),
                ListItem::new(
                    "The API key will be used only for this session and will not be stored permanently.",
                ),
                ListItem::new(
                    "You can also set the OPENAI_API_KEY environment variable to avoid this prompt.",
                ),
            ]
        ),
        _ => (
            "Anthropic API Key Setup",
            vec![
                ListItem::new("To use Claude 3.7, you need to provide your Anthropic API key."),
                ListItem::new("You can get an API key from https://console.anthropic.com/"),
                ListItem::new(""),
                ListItem::new(
                    "The API key will be used only for this session and will not be stored permanently.",
                ),
                ListItem::new(
                    "You can also set the ANTHROPIC_API_KEY environment variable to avoid this prompt.",
                ),
            ]
        ),
    };

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Messages area showing info about API key requirements
    let messages = List::new(message_items)
        .block(Block::default().borders(Borders::ALL).title("Information"))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(messages, chunks[1]);

    // Determine placeholder text based on model
    let placeholder = match app.current_model().name.as_str() {
        "GPT-4o" => "Enter your OpenAI API key and press Enter...",
        _ => "Enter your Anthropic API key and press Enter...",
    };

    // Input box with masked input for security
    let input_content = if app.input.is_empty() {
        Span::styled(placeholder, Style::default().fg(Color::DarkGray))
    } else {
        // Mask the API key with asterisks for privacy
        Span::raw("*".repeat(app.input.len()))
    };

    let input_box = Paragraph::new(input_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("API Key")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default())
        .alignment(Alignment::Left);
    f.render_widget(input_box, chunks[2]);

    // Set cursor position for input
    if !app.input.is_empty() {
        // Position the cursor at the end of the masked input
        f.set_cursor_position((chunks[2].x + app.input.len() as u16 + 1, chunks[2].y + 1));
    } else {
        // Position at the start of the input area
        f.set_cursor_position((chunks[2].x + 1, chunks[2].y + 1));
    }
}

fn draw_error(f: &mut Frame, _app: &App, error_msg: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("Error Occurred")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let error_text = Paragraph::new(error_msg)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error Details"),
        )
        .style(Style::default().fg(Color::Red))
        .wrap(Wrap { trim: true });
    f.render_widget(error_text, chunks[1]);

    let instruction = Paragraph::new("Press Enter to return to setup or Esc to exit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    f.render_widget(instruction, chunks[2]);
}
