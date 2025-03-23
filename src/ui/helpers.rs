use crate::app::models::ModelManager;
use crate::app::state::{App, TaskStatus};
use crate::ui::styles::AppStyles;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph},
};
use std::time::Duration;

/// Message type enumeration for consistent formatting
#[derive(Debug, PartialEq, Eq)]
enum MessageType {
    User,
    Tool,
    Thinking,
    Success,
    Error,
    Wait,
    Debug,
    Status,
    Title,
    Normal,
}

/// Indicator type for visual status representation
#[derive(Debug, PartialEq, Eq)]
enum IndicatorType {
    InProgress,
    Success,
    #[allow(dead_code)]
    Error,
    #[allow(dead_code)]
    Wait,
    #[allow(dead_code)]
    None,
}

/// Get animation state based on app timing
pub fn get_animation_state(app: &App) -> (bool, bool) {
    let animation_active = app.last_message_time.elapsed() < Duration::from_millis(1000);
    let highlight_on =
        animation_active && (std::time::Instant::now().elapsed().as_millis() % 500) < 300;
    (animation_active, highlight_on)
}

/// Process messages into styled lines
pub fn process_messages(
    messages: &[&String],
    animation_state: (bool, bool),
    debug_enabled: bool,
) -> Vec<Line<'static>> {
    let (_animation_active, highlight_on) = animation_state;
    let total_messages = messages.len();
    let mut all_lines = Vec::new();

    for (idx, message) in messages.iter().enumerate() {
        // Detect message type
        let message_type = detect_message_type(message);

        match message_type {
            MessageType::User => {
                add_user_message_lines(&mut all_lines, message, idx, total_messages, highlight_on)
            }
            MessageType::Debug => {
                if debug_enabled {
                    all_lines.push(Line::from(vec![Span::styled(
                        message.to_string(),
                        Style::default().fg(Color::Yellow),
                    )]));
                }
            }
            _ => {
                all_lines.push(format_message(
                    message,
                    idx,
                    total_messages,
                    highlight_on,
                    debug_enabled,
                ));
            }
        }
    }

    all_lines
}

/// Add user message to lines, handling multiline messages
fn add_user_message_lines(
    lines: &mut Vec<Line<'static>>,
    message: &str,
    _idx: usize,
    _total_messages: usize,
    _highlight_on: bool,
) {
    if let Some(stripped) = message.strip_prefix("> ") {
        if stripped.contains('\n') {
            // Handle multiline user message
            let message_lines: Vec<&str> = stripped.split('\n').collect();

            // Add first line with "YOU:" prefix
            lines.push(Line::from(vec![
                Span::styled(
                    "YOU: ",
                    Style::default()
                        .fg(AppStyles::accent_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(message_lines[0].to_string(), AppStyles::user_input()),
            ]));

            // Add remaining lines with proper indentation
            for line in &message_lines[1..] {
                lines.push(Line::from(vec![
                    Span::styled(
                        "     ", // Align with "YOU: "
                        Style::default()
                            .fg(AppStyles::accent_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled((*line).to_string(), AppStyles::user_input()),
                ]));
            }
        } else {
            // Single-line user message
            lines.push(Line::from(vec![
                Span::styled(
                    "YOU: ",
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(stripped.to_string(), AppStyles::user_input()),
            ]));
        }
    }
}

/// Detect the type of message from its content
fn detect_message_type(message: &str) -> MessageType {
    if message.starts_with("> ") {
        MessageType::User
    } else if message.contains("\x1b[32m⏺\x1b[0m")
        || message.contains("\x1b[31m⏺\x1b[0m")
        || message.starts_with("⏺ ")
        || message.starts_with("[tool] ")
    {
        MessageType::Tool
    } else if message.starts_with("[thinking]") || message == "Thinking..." {
        MessageType::Thinking
    } else if message.starts_with("[success] ") {
        MessageType::Success
    } else if message.starts_with("[error] ")
        || message.starts_with("Error:")
        || message.starts_with("ERROR:")
    {
        MessageType::Error
    } else if message.starts_with("[wait]") {
        MessageType::Wait
    } else if message.starts_with("DEBUG:") {
        MessageType::Debug
    } else if message.starts_with("Status:") {
        MessageType::Status
    } else if message.starts_with("★") {
        MessageType::Title
    } else {
        MessageType::Normal
    }
}

/// Format a message based on its detected type
pub fn format_message(
    message: &str,
    idx: usize,
    total_messages: usize,
    highlight_on: bool,
    debug_enabled: bool,
) -> Line<'static> {
    let is_newest_msg = idx == total_messages - 1;
    let message_type = detect_message_type(message);

    match message_type {
        MessageType::Tool => format_tool_message(message, is_newest_msg, highlight_on),
        MessageType::Thinking => format_thinking_message(message, is_newest_msg, highlight_on),
        MessageType::Success => format_success_message(message),
        MessageType::Wait => format_wait_message(message),
        MessageType::Error => format_error_message(message),
        MessageType::Debug => {
            if debug_enabled {
                Line::from(vec![Span::styled(
                    message.to_string(),
                    Style::default().fg(Color::Yellow),
                )])
            } else {
                Line::from("")
            }
        }
        MessageType::Status => Line::from(vec![Span::styled(
            message.to_string(),
            Style::default().fg(Color::Blue),
        )]),
        MessageType::Title => {
            Line::from(vec![Span::styled(message.to_string(), AppStyles::title())])
        }
        MessageType::User => {
            if let Some(stripped) = message.strip_prefix("> ") {
                Line::from(vec![
                    Span::styled(
                        "YOU: ",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(stripped.to_string(), AppStyles::user_input()),
                ])
            } else {
                Line::from(vec![Span::raw(message.to_string())])
            }
        }
        MessageType::Normal => format_model_response(message),
    }
}

/// Format tool messages with appropriate styling and indicators
fn format_tool_message(message: &str, is_newest_msg: bool, highlight_on: bool) -> Line<'static> {
    // Extract the content from various tool message formats
    let content = if message.contains("\x1b[32m⏺\x1b[0m") {
        if let Some(ansi_end_pos) = message.find("\x1b[0m") {
            let content_start = ansi_end_pos + 4; // Length of "\x1b[0m"
            if content_start < message.len() {
                message[content_start..].trim_start()
            } else {
                ""
            }
        } else {
            ""
        }
    } else if let Some(content) = message.strip_prefix("[tool] ") {
        content
    } else if let Some(content) = message.strip_prefix("⏺ ") {
        content
    } else {
        message
    };

    // Determine if this is a completed tool based on content
    let is_completed = content.contains("Result:") || content.contains("completed");

    // Get indicator style based on status and animation
    let indicator_style = get_indicator_style(
        if is_completed {
            IndicatorType::Success
        } else {
            IndicatorType::InProgress
        },
        is_newest_msg,
        highlight_on,
    );

    // Tool text style
    let text_style = Style::default()
        .fg(Color::LightBlue)
        .add_modifier(Modifier::BOLD);

    Line::from(vec![
        Span::styled("⏺ ", indicator_style),
        Span::styled(content.to_string(), text_style),
    ])
}

/// Format thinking messages with appropriate styling
fn format_thinking_message(
    message: &str,
    is_newest_msg: bool,
    highlight_on: bool,
) -> Line<'static> {
    // Extract content from thinking message
    let content = message
        .strip_prefix("[thinking] ")
        .or_else(|| message.strip_prefix("Thinking..."))
        .unwrap_or(message)
        .strip_prefix("⚪ ")
        .or_else(|| message.strip_prefix("⏺ "))
        .unwrap_or(message);

    // Get thinking text style based on animation state
    let text_style = if is_newest_msg && highlight_on {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::ITALIC)
    };

    // Get indicator style
    let indicator_style =
        get_indicator_style(IndicatorType::InProgress, is_newest_msg, highlight_on);

    Line::from(vec![
        Span::styled("⏺ ", indicator_style),
        Span::styled(content.to_string(), text_style),
    ])
}

/// Format success messages with green indicator
fn format_success_message(message: &str) -> Line<'static> {
    // Extract content from success message
    let content = if message.contains("\x1b[32m⏺\x1b[0m") {
        if let Some(ansi_end_pos) = message.find("\x1b[0m") {
            let content_start = ansi_end_pos + 4;
            if content_start < message.len() {
                message[content_start..].trim_start()
            } else {
                ""
            }
        } else {
            ""
        }
    } else if let Some(content) = message.strip_prefix("[success] ") {
        content
    } else if let Some(content) = message.strip_prefix("⏺ ") {
        content
    } else {
        message
    };

    Line::from(vec![
        Span::styled("⏺ ", Style::default().fg(Color::Green)),
        Span::styled(content.to_string(), Style::default().fg(Color::Green)),
    ])
}

/// Format wait messages with white circle indicator
fn format_wait_message(message: &str) -> Line<'static> {
    let content = message
        .strip_prefix("[wait] ")
        .unwrap_or(message)
        .strip_prefix("⚪ ")
        .unwrap_or(message);

    Line::from(vec![
        Span::styled("⚪ ", Style::default().fg(Color::LightYellow)),
        Span::styled(content.to_string(), Style::default().fg(Color::Yellow)),
    ])
}

/// Format error messages with red indicator
fn format_error_message(message: &str) -> Line<'static> {
    // Extract content from error message formats
    let content = if message.contains("\x1b[31m⏺\x1b[0m") {
        if let Some(ansi_end_pos) = message.find("\x1b[0m") {
            let content_start = ansi_end_pos + 4;
            if content_start < message.len() {
                message[content_start..].trim_start()
            } else {
                ""
            }
        } else {
            ""
        }
    } else if let Some(content) = message.strip_prefix("[error] ") {
        content
    } else if let Some(content) = message.strip_prefix("Error:") {
        content
    } else if let Some(content) = message.strip_prefix("ERROR:") {
        content
    } else {
        message
    };

    Line::from(vec![
        Span::styled("⏺ ", AppStyles::error()),
        Span::styled(content.to_string(), AppStyles::error()),
    ])
}

/// Format model responses
fn format_model_response(message: &str) -> Line<'static> {
    if message.trim().is_empty() {
        Line::from(" ") // Use a space instead of empty to reduce vertical spacing issues
    } else {
        Line::from(vec![Span::raw(message.to_string())])
    }
}

/// Get indicator style based on type and animation state
fn get_indicator_style(
    indicator_type: IndicatorType,
    is_newest: bool,
    highlight_on: bool,
) -> Style {
    match indicator_type {
        IndicatorType::Success => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        IndicatorType::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        IndicatorType::Wait => Style::default().fg(Color::LightYellow),
        IndicatorType::InProgress => {
            if is_newest && highlight_on {
                Style::default()
                    .fg(Color::Rgb(255, 165, 0)) // Orange
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::SLOW_BLINK)
            } else {
                Style::default().fg(Color::Rgb(255, 165, 0))
            }
        }
        IndicatorType::None => Style::default(),
    }
}

/// Apply scrolling to lines and determine if there are more lines above/below
pub fn apply_scrolling(
    lines: &[Line<'static>],
    scroll_position: usize,
    visible_height: usize,
) -> (Vec<Line<'static>>, bool, bool) {
    let total_lines = lines.len();

    let visible_lines = if total_lines <= visible_height {
        lines.to_vec()
    } else {
        lines
            .iter()
            .skip(scroll_position)
            .take(visible_height)
            .cloned()
            .collect()
    };

    let has_more_above = scroll_position > 0;
    let has_more_below = (scroll_position + visible_height) < total_lines;

    (visible_lines, has_more_above, has_more_below)
}

/// Create a block with scroll indicators
pub fn create_scrollable_block(
    title: &str,
    has_more_above: bool,
    has_more_below: bool,
    title_style: Style,
) -> Block<'static> {
    let block_title = if has_more_above && has_more_below {
        Line::from(vec![
            Span::styled(format!("{} ", title), title_style),
            Span::styled("▲ more above ", AppStyles::hint()),
            Span::styled("▼ more below", AppStyles::hint()),
        ])
    } else if has_more_above {
        Line::from(vec![
            Span::styled(format!("{} ", title), title_style),
            Span::styled("▲ more above", AppStyles::hint()),
        ])
    } else if has_more_below {
        Line::from(vec![
            Span::styled(format!("{} ", title), title_style),
            Span::styled("▼ more below", AppStyles::hint()),
        ])
    } else {
        Line::from(Span::styled(title.to_string(), title_style))
    };

    Block::default()
        .borders(Borders::ALL)
        .title(block_title)
        .title_alignment(Alignment::Left)
        .border_style(AppStyles::border())
        .padding(Padding::new(1, 1, 0, 0))
}

/// Create empty input content with optional placeholder
pub fn create_empty_input_content(placeholder: &str) -> Text<'static> {
    Text::from(vec![Line::from(vec![
        // Prompt is now rendered separately through add_prompt
        if !placeholder.is_empty() {
            Span::styled(placeholder.to_string(), AppStyles::hint())
        } else {
            Span::raw("")
        },
        Span::styled("█", AppStyles::cursor()),
    ])])
}

/// Create masked input content for API keys
pub fn create_masked_input_content(app: &App) -> Text<'static> {
    let (before_cursor, after_cursor) = if app.cursor_position < app.input.len() {
        (app.cursor_position, app.input.len() - app.cursor_position)
    } else {
        (app.input.len(), 0)
    };

    let mut spans = vec![];

    if before_cursor > 0 {
        spans.push(Span::raw("*".repeat(before_cursor)));
    }

    spans.push(Span::styled("█", AppStyles::cursor()));

    if after_cursor > 0 {
        spans.push(Span::raw("*".repeat(after_cursor)));
    }

    Text::from(vec![Line::from(spans)])
}

/// Create single-line input content with cursor
pub fn create_single_line_input_content(app: &App) -> Text<'static> {
    // Text style for normal content
    let text_style = Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD);

    // Split input at cursor position
    let (before_cursor, after_cursor) = app.input.split_at(app.cursor_position);
    let mut spans = vec![];

    // Add text before cursor
    if !before_cursor.is_empty() {
        spans.push(Span::styled(before_cursor.to_string(), text_style));
    }

    // Handle cursor positioning
    if app.cursor_position < app.input.len() {
        // We're at a character position
        if let Some(cursor_char) = after_cursor.chars().next() {
            // Split after_cursor into first character and rest
            let first_char = cursor_char.to_string();
            let rest_of_text = if after_cursor.len() > cursor_char.len_utf8() {
                after_cursor[cursor_char.len_utf8()..].to_string()
            } else {
                "".to_string()
            };

            // Add the character at cursor position with cursor style
            spans.push(Span::styled(first_char, AppStyles::cursor()));

            // Add the rest of the text with normal style
            if !rest_of_text.is_empty() {
                spans.push(Span::styled(rest_of_text, text_style));
            }
        }
    } else {
        // We're at the end of text, show a block cursor
        spans.push(Span::styled("█", AppStyles::cursor()));
    }

    Text::from(vec![Line::from(spans)])
}

/// Create multiline input content with cursor
pub fn create_multiline_input_content(app: &App) -> Text<'static> {
    // Define styles
    let text_style = Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD);

    // Split input into lines and process each one
    let input_str = app.input.as_str();
    let lines: Vec<&str> = input_str.split('\n').collect();
    let trailing_newline = input_str.ends_with('\n');

    // Convert to styled Lines
    let mut styled_lines = Vec::new();

    // Track position within the overall string
    let mut char_pos = 0;
    let cursor_pos = app.cursor_position.min(input_str.len());

    // Process each line
    for (idx, line) in lines.iter().enumerate() {
        let line_start_pos = char_pos;
        let line_end_pos = line_start_pos + line.len();

        // Check if cursor is on this line
        let cursor_on_this_line = cursor_pos >= line_start_pos
            && (cursor_pos <= line_end_pos
                || (idx == lines.len() - 1 && trailing_newline && cursor_pos == line_end_pos + 1));

        // Indent lines after the first one
        let line_padding = if idx == 0 { "" } else { "  " };

        if cursor_on_this_line {
            // Add line with cursor
            add_line_with_cursor(
                &mut styled_lines,
                line,
                line_padding,
                cursor_pos - line_start_pos,
                text_style,
            );
        } else {
            // Regular line without cursor
            let padding_span = if !line_padding.is_empty() {
                Span::raw(line_padding)
            } else {
                Span::raw("")
            };

            let mut spans = Vec::new();
            if !line_padding.is_empty() {
                spans.push(padding_span);
            }

            spans.push(Span::styled(line.to_string(), text_style));
            styled_lines.push(Line::from(spans));
        }

        // Update position (add 1 for the newline character)
        char_pos = line_end_pos + 1;
    }

    // Handle trailing newline with cursor
    if trailing_newline && cursor_pos == input_str.len() {
        styled_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("█", AppStyles::cursor()),
        ]));
    } else if trailing_newline {
        // Add empty line for trailing newline without cursor
        styled_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("", text_style),
        ]));
    }

    Text::from(styled_lines)
}

/// Add a line with cursor at specified position
fn add_line_with_cursor(
    styled_lines: &mut Vec<Line<'static>>,
    line: &str,
    line_padding: &str,
    cursor_offset: usize,
    text_style: Style,
) {
    if cursor_offset <= line.len() {
        // Regular cursor position within the line
        let (before_cursor, after_cursor) = line.split_at(cursor_offset);

        // Create spans vector
        let mut spans = Vec::new();

        // Add padding for indentation if needed
        if !line_padding.is_empty() {
            spans.push(Span::raw(line_padding.to_string()));
        }

        // Add text before cursor
        if !before_cursor.is_empty() {
            spans.push(Span::styled(before_cursor.to_string(), text_style));
        }

        // Handle cursor and text after
        if !after_cursor.is_empty() {
            if let Some(cursor_char) = after_cursor.chars().next() {
                // Split after_cursor into first character and rest
                let first_char = cursor_char.to_string();
                let rest_of_text = if after_cursor.len() > cursor_char.len_utf8() {
                    after_cursor[cursor_char.len_utf8()..].to_string()
                } else {
                    "".to_string()
                };

                // Add character at cursor with cursor style
                spans.push(Span::styled(first_char, AppStyles::cursor()));

                // Add rest of text with normal style
                if !rest_of_text.is_empty() {
                    spans.push(Span::styled(rest_of_text, text_style));
                }
            } else {
                // End of line with no characters
                spans.push(Span::styled("█", AppStyles::cursor()));
            }
        } else {
            // At the end of text, just add block cursor
            spans.push(Span::styled("█", AppStyles::cursor()));
        }

        styled_lines.push(Line::from(spans));
    } else {
        // This handles the special case at end of line with trailing newline
        let mut spans = Vec::new();

        // Add padding for indentation if needed
        if !line_padding.is_empty() {
            spans.push(Span::raw(line_padding.to_string()));
        }

        // Add line text with cursor at the end
        spans.push(Span::styled(line.to_string(), text_style));
        spans.push(Span::styled("█", AppStyles::cursor()));

        styled_lines.push(Line::from(spans));
    }
}

/// Get input placeholder based on app mode
pub fn get_input_placeholder(app: &App, is_api_key: bool) -> &'static str {
    if is_api_key {
        match app.current_model().name.as_str() {
            "GPT-4o" => "Enter your OpenAI API key and press Enter...",
            _ => "Enter your Anthropic API key and press Enter...",
        }
    } else {
        "" // Empty placeholder for regular input
    }
}

/// Add task lines to the given vector
pub fn add_task_lines(
    lines: &mut Vec<Line<'static>>,
    task: &crate::app::state::Task,
    animation_state: (bool, bool),
    width: u16,
) {
    let (_animation_active, highlight_on) = animation_state;

    // Format the task status indicator
    let (indicator, style) = match &task.status {
        TaskStatus::InProgress => {
            if highlight_on {
                (
                    "⏺",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::SLOW_BLINK),
                )
            } else {
                ("⏺", Style::default().fg(Color::White))
            }
        }
        TaskStatus::Completed {
            duration: _,
            tool_uses: _,
            input_tokens: _,
            output_tokens: _,
        } => (
            "⏺",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        TaskStatus::Failed(_) => (
            "⏺",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    };

    // Add main task line with description
    lines.push(Line::from(vec![
        Span::styled(indicator, style),
        Span::raw(" "),
        Span::styled(
            truncate_with_ellipsis(&task.description, width.saturating_sub(10) as usize),
            Style::default().fg(Color::LightCyan),
        ),
    ]));

    // Add task details line with status information
    match &task.status {
        TaskStatus::InProgress => {
            lines.push(Line::from(vec![
                Span::raw("  ⎿ "),
                Span::styled(
                    format!(
                        "In progress ({} tool use{})",
                        task.tool_count,
                        if task.tool_count == 1 { "" } else { "s" }
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        TaskStatus::Completed {
            duration,
            tool_uses,
            input_tokens,
            output_tokens,
        } => {
            // Format duration as seconds with one decimal place
            let duration_secs = duration.as_secs_f32();
            let total_tokens = input_tokens + output_tokens;

            lines.push(Line::from(vec![
                Span::raw("  ⎿ "),
                Span::styled(
                    format!(
                        "Done ({} tool use{} · {:.1}k tokens [{:.1}k in/{:.1}k out] · {:.1}s)",
                        tool_uses,
                        if *tool_uses == 1 { "" } else { "s" },
                        total_tokens as f32 / 1000.0,
                        *input_tokens as f32 / 1000.0,
                        *output_tokens as f32 / 1000.0,
                        duration_secs
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        TaskStatus::Failed(error) => {
            lines.push(Line::from(vec![
                Span::raw("  ⎿ "),
                Span::styled(
                    format!(
                        "Failed: {}",
                        truncate_with_ellipsis(error, width.saturating_sub(15) as usize)
                    ),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }
    }

    // Add space between tasks
    lines.push(Line::from(""));
}

/// Create detailed keyboard shortcuts panel
pub fn create_detailed_shortcuts() -> Paragraph<'static> {
    // Define shortcuts and their descriptions
    let shortcuts = [
        ("/ ", "Show commands menu"),
        ("Ctrl+j", "Add newline in input"),
    ];

    // Calculate max shortcut length for alignment
    let max_shortcut_length = shortcuts.iter().map(|(s, _)| s.len()).max().unwrap_or(0);

    // Create the title and shortcut lines
    let mut lines = vec![Line::from(vec![Span::styled(
        "Keyboard Shortcuts",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )])];

    // Add each shortcut with proper alignment
    for (shortcut, description) in shortcuts.iter() {
        let padding = " ".repeat(max_shortcut_length.saturating_sub(shortcut.len()) + 2);
        lines.push(Line::from(vec![
            Span::styled(
                (*shortcut).to_string(),
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(padding, Style::default()),
            Span::styled(
                (*description).to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    Paragraph::new(Text::from(lines))
}

/// Helper function to truncate a string if it's too long
pub fn truncate_with_ellipsis(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        s.to_string()
    } else {
        format!("{}{}", &s[0..max_length.saturating_sub(3)], "...")
    }
}
