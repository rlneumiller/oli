use crate::app::commands::CommandHandler;
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
    let scroll_info = format!(
        "Scroll: {}/{}",
        app.scroll_position,
        app.messages.len().saturating_sub(10)
    );

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

    // Update scroll state and apply scrolling
    app.message_scroll
        .update_dimensions(all_lines.len(), visible_area.height as usize);
    app.scroll_position = app.message_scroll.position;

    let (visible_messages, has_more_above, has_more_below) = helpers::apply_scrolling(
        &all_lines,
        app.message_scroll.position,
        visible_area.height as usize,
    );

    // Create block with scroll indicators
    let message_block = helpers::create_scrollable_block(
        "OLI Assistant",
        has_more_above,
        has_more_below,
        AppStyles::section_header(),
    );

    // Create paragraph with the styled messages
    Paragraph::new(Text::from(visible_messages))
        .block(message_block)
        .wrap(Wrap { trim: false })
        .scroll((0, 0)) // Prevent auto-scrolling issues
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

/// Create a task list with animated status indicators
pub fn create_task_list(app: &mut App, visible_area: Rect) -> Paragraph<'static> {
    // Get animation state for blinking effects
    let animation_state = helpers::get_animation_state(app);

    // Process tasks into styled lines
    let mut all_lines = Vec::new();

    // Add title
    all_lines.push(Line::from(vec![Span::styled(
        "Tasks",
        Style::default()
            .fg(AppStyles::primary_color())
            .add_modifier(Modifier::BOLD),
    )]));

    // Add an empty line after title
    all_lines.push(Line::from(""));

    // Process visible tasks (most recent ones)
    let visible_tasks = app.tasks.iter().rev().take(10).collect::<Vec<_>>();

    if visible_tasks.is_empty() {
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
                .title("Tasks")
                .title_alignment(Alignment::Left)
                .border_style(AppStyles::border())
                .padding(Padding::new(1, 1, 0, 0)),
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
