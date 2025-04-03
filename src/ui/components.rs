use crate::app::commands::CommandHandler;
// LogLevel is defined in logger.rs but not needed here
use crate::app::models::ModelManager;
use crate::app::state::App;
use crate::ui::helpers;
use crate::ui::styles::AppStyles;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph, Wrap},
};

/// Create a status bar for the chat view
pub fn create_status_bar(app: &App) -> Line<'static> {
    let model_name = app.current_model().name.clone();
    let version = env!("CARGO_PKG_VERSION");

    // Determine scroll counts based on what's currently being shown
    let scroll_info = if app.show_logs {
        format!(
            "Scroll: {}/{}",
            app.log_scroll.position,
            app.logs.len().saturating_sub(10)
        )
    } else {
        format!(
            "Scroll: {}/{}",
            app.scroll_position,
            app.messages.len().saturating_sub(10)
        )
    };

    // Add agent indicator based on agent availability
    let agent_indicator = if app.use_agent && app.agent.is_some() {
        Span::styled(
            " 🤖 Agent ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(142, 192, 124))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " 🖥️ Local ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(240, 180, 100)),
        )
    };

    // Debug indicator removed - we only need the LOGS view indicator

    // View mode indicator (logs or conversation)
    let view_mode = if app.show_logs {
        Span::styled(
            " LOGS ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(80, 80, 200))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("", Style::default())
    };

    Line::from(vec![
        Span::styled(
            format!(" oli v{} ", version),
            Style::default()
                .fg(Color::Black)
                .bg(AppStyles::primary_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {} ", model_name),
            Style::default()
                .fg(AppStyles::secondary_color())
                .bg(AppStyles::selection_bg()),
        ),
        Span::raw(" "),
        agent_indicator,
        Span::raw(" "),
        view_mode, // Only show the LOGS indicator
        Span::raw(" | "),
        Span::styled(scroll_info, AppStyles::hint()),
        Span::raw(" | "),
        Span::styled(" Esc: Quit ", AppStyles::status_bar()),
    ])
}

/// Create a chat history view with proper message formatting
pub fn create_message_list(app: &mut App, visible_area: Rect) -> Paragraph<'static> {
    // Filter invisible markers
    let display_messages: Vec<&String> = app
        .messages
        .iter()
        .filter(|msg| *msg != "_AUTO_SCROLL_")
        .collect();

    // Animation state for blinking effects
    let animation_state = helpers::get_animation_state(app);

    // Process messages into styled lines
    let all_lines =
        helpers::process_messages(&display_messages, animation_state, app.debug_messages);

    // Calculate available height for content (accounting for block borders and padding)
    // Subtract borders (2) and additional padding (1) to ensure we have the correct viewport size
    let available_height = visible_area.height.saturating_sub(3) as usize; // -3 for borders and padding

    // Store the previous content size for comparison
    let previous_content_size = app.message_scroll.content_size;

    // Update scroll state dimensions with actual content size
    app.message_scroll
        .update_dimensions(all_lines.len(), available_height);

    // If content size increased significantly and we're following the bottom,
    // ensure we're really at the absolute bottom
    if app.message_scroll.follow_bottom && all_lines.len() > previous_content_size + 1 {
        app.message_scroll.position = app.message_scroll.max_scroll();
    }

    // Update legacy scroll position for compatibility
    app.scroll_position = app.message_scroll.position;

    // Calculate which messages are visible in the viewport
    let (visible_messages, has_more_above, has_more_below) =
        helpers::apply_scrolling(&all_lines, app.message_scroll.position, available_height);

    // Create block with scroll indicators
    let message_block = helpers::create_scrollable_block(
        "oli Assistant",
        has_more_above,
        has_more_below,
        AppStyles::section_header(),
    );

    // Create paragraph with the styled messages
    Paragraph::new(Text::from(visible_messages))
        .block(message_block)
        .wrap(Wrap { trim: false })
        .scroll((0, 0)) // Prevent automatic scrolling
}

/// Create an input box for chat or API key input
#[allow(dead_code)]
pub fn create_input_box(app: &App, is_api_key: bool) -> Paragraph<'static> {
    // Determine input mode context
    let title = if is_api_key {
        "API Key"
    } else {
        "Input (Type / for commands)"
    };
    let placeholder = helpers::get_input_placeholder(app, is_api_key);

    // Generate appropriate content based on input state
    let input_content = if app.input.is_empty() {
        helpers::create_empty_input_content(placeholder)
    } else if is_api_key {
        helpers::create_masked_input_content(app)
    } else if !app.input.contains('\n') {
        helpers::create_single_line_input_content(app)
    } else {
        helpers::create_multiline_input_content(app)
    };

    Paragraph::new(input_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .title_alignment(Alignment::Left)
                .border_style(AppStyles::border()),
        )
        .wrap(Wrap { trim: false })
}

/// Create a command menu list for selection
pub fn create_command_menu(app: &App) -> List<'static> {
    let filtered_commands = app.filtered_commands();

    // Ensure selected command is in bounds
    let valid_selected = if filtered_commands.is_empty() {
        0
    } else {
        app.selected_command.min(filtered_commands.len() - 1)
    };

    // Calculate maximum command name length for alignment
    let max_cmd_length = filtered_commands
        .iter()
        .map(|cmd| cmd.name.len())
        .max()
        .unwrap_or(0);

    let command_items: Vec<ListItem> = filtered_commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let padding = " ".repeat(max_cmd_length.saturating_sub(cmd.name.len()) + 4);

            if i == valid_selected {
                ListItem::new(format!("▶ {}{}{}", cmd.name, padding, cmd.description))
                    .style(AppStyles::command_highlight())
            } else {
                ListItem::new(format!("  {}{}{}", cmd.name, padding, cmd.description))
                    .style(Style::default().fg(Color::DarkGray))
            }
        })
        .collect();

    List::new(command_items)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(AppStyles::command_highlight())
}

/// Create a list of models for selection in setup mode
pub fn create_model_list(app: &App) -> List<'static> {
    let models: Vec<ListItem> = app
        .available_models
        .iter()
        .enumerate()
        .map(|(i, model)| {
            let content = format!("{} - {}", model.name, model.description);
            if i == app.selected_model {
                ListItem::new(format!("→ {}", content)).style(AppStyles::highlight())
            } else {
                ListItem::new(format!("  {}", content))
            }
        })
        .collect();

    List::new(models)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Models ")
                .title_alignment(Alignment::Left)
                .border_style(AppStyles::border())
                .padding(Padding::new(1, 0, 1, 0)),
        )
        .highlight_style(AppStyles::highlight())
}

/// Create a progress display for model setup
pub fn create_progress_display(_app: &App) -> Paragraph<'static> {
    let progress_text: String = "Press Enter to begin setup".to_string();

    Paragraph::new(progress_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Progress ")
                .title_alignment(Alignment::Left)
                .border_style(AppStyles::border())
                .padding(Padding::new(1, 0, 0, 0)),
        )
        .style(AppStyles::success())
}

/// Create an information display for API key setup
pub fn create_api_key_info(app: &App) -> List<'static> {
    // Determine message items based on selected model
    let message_items = match app.current_model().name.as_str() {
        "GPT-4o" => vec![
            ListItem::new("To use GPT-4o, you need to provide your OpenAI API key."),
            ListItem::new("You can get an API key from https://platform.openai.com/api-keys"),
            ListItem::new(""),
            ListItem::new(
                "The API key will be used only for this session and will not be stored permanently.",
            ),
            ListItem::new(
                "You can also set the OPENAI_API_KEY environment variable to avoid this prompt.",
            ),
        ],
        name if name.contains("Local") => vec![
            ListItem::new("Local Ollama models don't require API keys."),
            ListItem::new("Make sure Ollama is running with 'ollama serve'"),
            ListItem::new(""),
            ListItem::new("Press Enter to continue without an API key."),
            ListItem::new("If you're seeing this screen, there may be a bug in the application."),
        ],
        _ => vec![
            ListItem::new("To use Claude 3.7, you need to provide your Anthropic API key."),
            ListItem::new("You can get an API key from https://console.anthropic.com/"),
            ListItem::new(""),
            ListItem::new(
                "The API key will be used only for this session and will not be stored permanently.",
            ),
            ListItem::new(
                "You can also set the ANTHROPIC_API_KEY environment variable to avoid this prompt.",
            ),
        ],
    };

    List::new(message_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Information ")
                .title_alignment(Alignment::Left)
                .border_style(AppStyles::border())
                .padding(Padding::new(1, 0, 0, 0)),
        )
        .style(Style::default().fg(Color::Rgb(240, 180, 100)))
}

/// Create a permission dialog
pub fn create_permission_dialog(_app: &App, _area: Rect) -> Block<'static> {
    Block::default()
        .title(" Permission Required ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(240, 180, 100)))
        .padding(Padding::new(1, 1, 0, 0))
}

/// Create permission dialog content
pub fn create_permission_content(app: &App) -> Paragraph<'static> {
    let tool = app.pending_tool.as_ref().unwrap();
    let tool_name = tool.tool_name.clone();
    let description = tool.description.to_string();

    let info_text = Text::from(vec![
        Line::from(vec![
            Span::styled("⚠️  ", Style::default().fg(Color::Rgb(240, 180, 100))),
            Span::styled(
                "Permission Required",
                Style::default()
                    .fg(Color::Rgb(240, 180, 100))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Tool: "),
            Span::styled(tool_name, Style::default().fg(AppStyles::primary_color())),
        ]),
        Line::from(vec![
            Span::raw("Action: "),
            Span::styled(
                description,
                Style::default().fg(AppStyles::secondary_color()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Y to allow or N to deny",
            AppStyles::hint(),
        )]),
    ]);

    Paragraph::new(info_text)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
}

/// Create a log display view with highlighted log entries
pub fn create_log_list(app: &mut App, visible_area: Rect) -> Paragraph<'_> {
    // Process logs into styled lines
    let mut all_lines = Vec::new();

    // Process visible logs (most recent ones)
    if app.logs.is_empty() {
        // Add some spacing at the top
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![Span::styled(
            "No logs yet. Logs will appear here when debug mode is enabled.",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        // Add all log entries with color-coded levels
        for log in &app.logs {
            let line = if log.contains(" [DEBUG] ") {
                Line::from(vec![Span::styled(
                    log,
                    Style::default().fg(Color::Rgb(120, 180, 180)), // Cyan for DEBUG
                )])
            } else if log.contains(" [INFO] ") {
                Line::from(vec![Span::styled(
                    log,
                    Style::default().fg(Color::Rgb(100, 180, 100)), // Green for INFO
                )])
            } else if log.contains(" [WARN] ") {
                Line::from(vec![Span::styled(
                    log,
                    Style::default().fg(Color::Rgb(230, 180, 80)), // Yellow for WARN
                )])
            } else if log.contains(" [ERROR] ") {
                Line::from(vec![Span::styled(
                    log,
                    Style::default().fg(Color::Rgb(220, 60, 60)), // Red for ERROR
                )])
            } else {
                // Default styling for unrecognized log format
                Line::from(vec![Span::styled(log, Style::default().fg(Color::White))])
            };

            all_lines.push(line);
        }
    }

    // Update scroll state
    app.log_scroll
        .update_dimensions(all_lines.len(), visible_area.height as usize);

    // Apply scrolling
    let visible_lines = if all_lines.len() <= visible_area.height as usize {
        all_lines
    } else {
        all_lines
            .into_iter()
            .skip(app.log_scroll.position)
            .take(visible_area.height as usize)
            .collect()
    };

    // Create a styled log list
    Paragraph::new(Text::from(visible_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Debug Logs ")
                .title_alignment(Alignment::Left)
                .border_style(Style::default().fg(Color::Rgb(100, 150, 255)))
                .padding(Padding::new(1, 0, 1, 0)),
        )
        .wrap(Wrap { trim: true })
}

/// Create a task list with animated status indicators
pub fn create_task_list(app: &mut App, visible_area: Rect) -> Paragraph<'static> {
    // Get animation state for blinking effects
    let animation_state = helpers::get_animation_state(app);

    // Process tasks into styled lines
    let mut all_lines = Vec::new();

    // Process visible tasks (most recent ones)
    let visible_tasks = app.tasks.iter().rev().take(10).collect::<Vec<_>>();

    if visible_tasks.is_empty() {
        // Add some spacing at the top
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![Span::styled(
            "No tasks yet. Type a query to get started.",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for task in visible_tasks {
            helpers::add_task_lines(&mut all_lines, task, animation_state, visible_area.width);
        }
    }

    // Update scroll state
    app.task_scroll
        .update_dimensions(all_lines.len(), visible_area.height as usize);
    app.task_scroll_position = app.task_scroll.position;

    // Apply scrolling
    let visible_lines = if all_lines.len() <= visible_area.height as usize {
        all_lines
    } else {
        all_lines
            .into_iter()
            .skip(app.task_scroll.position)
            .take(visible_area.height as usize)
            .collect()
    };

    // Create a styled task list
    Paragraph::new(Text::from(visible_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tasks ")
                .title_alignment(Alignment::Left)
                .border_style(AppStyles::border())
                .padding(Padding::new(1, 0, 1, 0)),
        )
        .wrap(Wrap { trim: false })
}

/// Create a shortcuts panel for display below the input box
pub fn create_shortcuts_panel(app: &App) -> Paragraph<'static> {
    if !app.input.is_empty() {
        return Paragraph::new("");
    }

    if app.show_detailed_shortcuts {
        helpers::create_detailed_shortcuts()
    } else if app.show_shortcuts_hint {
        Paragraph::new(Text::from(vec![Line::from(vec![
            Span::styled(
                "? ",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("for shortcuts", Style::default().fg(Color::DarkGray)),
        ])]))
    } else {
        Paragraph::new("")
    }
}
