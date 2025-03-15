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
    initialize_messages(&mut app);
    app.messages
        .push("DEBUG: Application started. Press Enter to begin setup.".into());

    let (tx, rx) = std::sync::mpsc::channel::<String>();

    terminal.draw(|f| ui(f, &app))?;

    while app.state != AppState::Error("quit".into()) {
        // Always redraw if download is active, to show progress
        if app.download_active {
            terminal.draw(|f| ui(f, &app))?;
        }

        // Check for messages from download thread
        while let Ok(msg) = rx.try_recv() {
            app.messages
                .push(format!("DEBUG: Received message: {}", msg));
            process_message(&mut app, &msg)?;
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
                            AppState::Chat => {
                                let input = std::mem::take(&mut app.input);
                                if !input.is_empty() {
                                    app.messages.push(format!("> {}", input));

                                    // Show a "thinking" message
                                    app.messages.push("Thinking...".into());
                                    terminal.draw(|f| ui(f, &app))?;

                                    // Query the model
                                    match app.query_model(&input) {
                                        Ok(response) => {
                                            // Remove the thinking message
                                            if let Some(last) = app.messages.last() {
                                                if last == "Thinking..." {
                                                    app.messages.pop();
                                                }
                                            }
                                            app.messages.push(response);
                                        }
                                        Err(e) => app.messages.push(format!("Error: {}", e)),
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
                    KeyCode::Down | KeyCode::Tab => {
                        if let AppState::Setup = app.state {
                            app.select_next_model();
                            app.messages.push("DEBUG: Selected next model".into());
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    KeyCode::Up | KeyCode::BackTab => {
                        if let AppState::Setup = app.state {
                            app.select_prev_model();
                            app.messages.push("DEBUG: Selected previous model".into());
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    KeyCode::Char(c) => {
                        if let AppState::Chat = app.state {
                            app.input.push(c);
                            terminal.draw(|f| ui(f, &app))?;
                        }
                    }
                    KeyCode::Backspace => {
                        if let AppState::Chat = app.state {
                            app.input.pop();
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
        let model_path = App::models_dir()?.join(&app.current_model().file_name);
        match app.load_model(&model_path) {
            Ok(()) => {
                app.state = AppState::Chat;
                app.messages.push("Setup complete. Ready to chat!".into());
                app.messages
                    .push("You can now ask questions about coding and development.".into());
            }
            Err(e) => {
                app.messages
                    .push(format!("ERROR: Failed to load model: {}", e));
                app.state = AppState::Error(format!("Failed to load model: {}", e));
            }
        }
    } else if msg == "setup_complete" {
        app.state = AppState::Chat;
        app.messages.push("Setup complete. Ready to chat!".into());
    } else if msg == "setup_failed" {
        app.messages
            .push("Setup failed. Check error messages above.".into());
    } else if msg.starts_with("error:") {
        let error_msg = msg.replacen("error:", "", 1);
        app.error_message = Some(error_msg.clone());
        app.state = AppState::Error(error_msg);
    } else if msg.starts_with("retry:") {
        app.messages.push(msg.replacen("retry:", "", 1));
    }

    Ok(())
}

fn initialize_messages(app: &mut App) {
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
        AppState::Chat => draw_chat(f, app),
        AppState::Error(ref error_msg) => draw_error(f, app, error_msg),
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    // Chat history
    let messages: Vec<Line> = app
        .messages
        .iter()
        .map(|m| {
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
            } else if m.starts_with("> ") {
                // User messages - cyan
                Line::from(vec![Span::styled(
                    m.as_str(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )])
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
            } else {
                // Regular text or model responses - white/default
                Line::from(m.as_str())
            }
        })
        .collect();

    let messages_window = Paragraph::new(Text::from(messages))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("OLI Assistant"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(messages_window, chunks[0]);

    // Input box with hint text
    let input_text = if app.input.is_empty() {
        Span::styled(
            "Type your code question and press Enter...",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::raw(app.input.as_str())
    };

    let input_window = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input (Esc to quit)")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(input_window, chunks[1]);

    // Only show cursor if there is input
    if !app.input.is_empty() {
        // Set cursor position at end of input
        f.set_cursor_position((chunks[1].x + app.input.width() as u16 + 1, chunks[1].y + 1));
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
