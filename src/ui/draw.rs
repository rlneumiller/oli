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
        f.set_cursor_position((chunks[2].x + app.input.len() as u16 + 3, chunks[2].y + 1));
    // +3: +1 for border, +2 for "> " prefix
    } else {
        // Position at the start of the input area
        f.set_cursor_position((chunks[2].x + 3, chunks[2].y + 1)); // +3: +1 for border, +2 for "> " prefix
    }
}

/// Draw chat screen with message history and input
pub fn draw_chat(f: &mut Frame, app: &App) {
    // Use three chunks - header, message history, and input (with command menu if active)

    // Count newlines in input to determine input box height
    let newline_count = app.input.chars().filter(|&c| c == '\n').count();

    // Calculate base input height (min 3, grows with newlines but caps at half the available height)
    // First, estimate the total available height (area height minus margins and other UI elements)
    let estimated_available_height = f.area().height.saturating_sub(4); // Subtract margins and status bar
    let max_input_height = estimated_available_height / 2; // Allow up to half the available height

    // Base height starts at 3 lines and grows with newlines, up to half the screen height
    let base_input_height = (3 + newline_count).min(max_input_height as usize);

    let input_height = if app.show_command_menu {
        // Increase the input area height to make room for the command menu
        let cmd_count = app.filtered_commands().len();
        // Add command menu height (up to 5) to the base input height
        base_input_height + cmd_count.min(5)
    } else {
        base_input_height // Use calculated height based on content
    };

    // Calculate height for shortcuts area - only show when input is empty
    let shortcuts_height = if app.input.is_empty() {
        if app.show_detailed_shortcuts {
            4 // Height for detailed shortcuts panel (increased for new shortcut)
        } else if app.show_shortcuts_hint {
            1 // Height for shortcut hint
        } else {
            0 // No height when shortcuts are disabled
        }
    } else {
        0 // No height when anything is typed in the input
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),                   // Status bar
            Constraint::Min(3),                      // Chat history (expandable)
            Constraint::Length(input_height as u16), // Input area (with variable height for command menu)
            Constraint::Length(shortcuts_height),    // Shortcuts area (variable height)
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

        // Set cursor position at end of input, handling multiline input
        if !app.input.is_empty() {
            let (cursor_x, cursor_y) =
                calculate_cursor_position(&app.input, input_chunks[0].x, input_chunks[0].y);
            f.set_cursor_position((cursor_x, cursor_y));
        }
    } else {
        // Regular input box without command menu
        let input_window = create_input_box(app, false);
        f.render_widget(input_window, chunks[2]);

        // Only show cursor if there is input
        if !app.input.is_empty() {
            // Set cursor position at end of input, handling multiline input
            let (cursor_x, cursor_y) =
                calculate_cursor_position(&app.input, chunks[2].x, chunks[2].y);
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // Render shortcuts panel if needed
    if shortcuts_height > 0 {
        let shortcuts_panel = create_shortcuts_panel(app);
        f.render_widget(shortcuts_panel, chunks[3]);
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

/// Calculate cursor position for multiline input
fn calculate_cursor_position(input: &str, base_x: u16, base_y: u16) -> (u16, u16) {
    if !input.contains('\n') {
        // Single line input
        // +1 for border, +2 for "> " prefix
        return (base_x + input.len() as u16 + 3, base_y + 1);
    }

    // For multiline input, place cursor at the end of the last line
    // Check if input ends with newline
    let trailing_newline = input.ends_with('\n');

    // Split the input by newlines
    let lines: Vec<&str> = input.split('\n').collect();

    // Determine the position of the cursor
    let line_count = lines.len();

    // Calculate the last line index (always the last line)
    let last_line_idx = line_count - 1;

    // Get the last line text
    let last_line = if trailing_newline {
        "" // Empty string for newline
    } else {
        lines.last().unwrap_or(&"")
    };

    // Cursor x position depends on whether we're on the first line or subsequent lines
    let indent_width = 2; // Width of the indentation ("  " or "> ")
    let border_offset = 1; // Offset for the border

    // Set x position at start of line + indentation + text length
    let x = base_x + border_offset + indent_width + last_line.len() as u16;

    // Set y position (add 1 for 0-indexed lines and 1 for the top border)
    let y = base_y + 1 + last_line_idx as u16;

    (x, y)
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
