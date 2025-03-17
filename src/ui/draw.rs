use crate::app::commands::CommandHandler;
use crate::app::models::ModelManager;
use crate::app::state::{App, AppState};
use crate::ui::components::*;
use crate::ui::styles::AppStyles;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Main UI rendering function, dispatches to specific screen renderers
pub fn ui(f: &mut Frame, app: &App) {
    match app.state {
        AppState::Setup => draw_setup(f, app),
        AppState::ApiKeyInput => draw_api_key_input(f, app),
        AppState::Chat => draw_chat(f, app),
        AppState::Error(ref error_msg) => draw_error(f, app, error_msg),
    }

    // Note: We no longer need to mutate app here as this is handled
    // in the events.rs file before calling ui.

    // Draw permission dialog over other UI elements when needed
    if app.permission_required && app.pending_tool.is_some() {
        draw_permission_dialog(f, app);
    }
}

/// Draw setup screen with model selection
pub fn draw_setup(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("OLI Setup Assistant")
        .style(AppStyles::title())
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Model list
    let models_list = create_model_list(app);
    f.render_widget(models_list, chunks[1]);

    // Progress
    let progress_bar = create_progress_display(app);
    f.render_widget(progress_bar, chunks[2]);
}

/// Draw API key input screen
pub fn draw_api_key_input(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Determine title based on selected model
    let title_text = match app.current_model().name.as_str() {
        "GPT-4o" => "OpenAI API Key Setup",
        _ => "Anthropic API Key Setup",
    };

    let title = Paragraph::new(title_text)
        .style(AppStyles::title())
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Information area showing API key requirements
    let info = create_api_key_info(app);
    f.render_widget(info, chunks[1]);

    // Input box
    let input_box = create_input_box(app, true);
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

/// Draw chat screen with message history and input
pub fn draw_chat(f: &mut Frame, app: &App) {
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
    let status_bar = create_status_bar(app);
    let status_bar_widget = Paragraph::new(status_bar).style(Style::default());
    f.render_widget(status_bar_widget, chunks[0]);

    // Messages history
    let messages_widget = create_message_list(app, chunks[1]);
    f.render_widget(messages_widget, chunks[1]);

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

        // Input box
        let input_window = create_input_box(app, false);
        f.render_widget(input_window, input_chunks[0]);

        // Commands menu as a list
        let commands_list = create_command_menu(app);
        f.render_widget(commands_list, input_chunks[1]);

        // Set cursor position at end of input
        if !app.input.is_empty() {
            f.set_cursor_position((
                input_chunks[0].x + app.input.len() as u16 + 1,
                input_chunks[0].y + 1,
            ));
        }
    } else {
        // Regular input box without command menu
        let input_window = create_input_box(app, false);
        f.render_widget(input_window, chunks[2]);

        // Only show cursor if there is input
        if !app.input.is_empty() {
            // Set cursor position at end of input
            f.set_cursor_position((chunks[2].x + app.input.len() as u16 + 1, chunks[2].y + 1));
        }
    }
}

/// Draw error screen
pub fn draw_error(f: &mut Frame, _app: &App, error_msg: &str) {
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
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(error_text, chunks[1]);

    let instruction = Paragraph::new("Press Enter to return to setup or Esc to exit")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    f.render_widget(instruction, chunks[2]);
}

/// Draw permission dialog over the current UI
pub fn draw_permission_dialog(f: &mut Frame, app: &App) {
    // Calculate dialog size and position (centered)
    let area = f.area();
    let width = std::cmp::min(70, area.width.saturating_sub(4));
    let height = 8;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;

    let dialog_area = Rect::new(x, y, width, height);

    // Create the dialog content
    let dialog = create_permission_dialog(app, dialog_area);

    f.render_widget(dialog, dialog_area);

    // Create content inside the dialog
    let inner_area = Rect {
        x: dialog_area.x + 1,
        y: dialog_area.y + 1,
        width: dialog_area.width.saturating_sub(2),
        height: dialog_area.height.saturating_sub(2),
    };

    // Display tool info
    let info = create_permission_content(app);

    f.render_widget(info, inner_area);
}
