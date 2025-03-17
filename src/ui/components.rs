use crate::app::commands::CommandHandler;
use crate::app::models::ModelManager;
use crate::app::state::App;
use crate::ui::styles::AppStyles;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::time::Duration;

/// Create a status bar for the chat view
pub fn create_status_bar(app: &App) -> Line {
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

    Line::from(vec![
        Span::styled(
            format!(" Model: {} ", model_name),
            Style::default().fg(Color::LightCyan).bg(Color::DarkGray),
        ),
        Span::raw(" "),
        agent_indicator,
        Span::raw(" | "),
        Span::styled(scroll_info, Style::default().fg(Color::DarkGray)),
        Span::raw(" | "),
        Span::styled(" Esc: Quit ", AppStyles::status_bar()),
    ])
}

/// Create a chat history view with proper message formatting
pub fn create_message_list(app: &App, visible_area: Rect) -> Paragraph {
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
    let highlight_on =
        animation_active && (std::time::Instant::now().elapsed().as_millis() % 500) < 300; // blink pattern

    // Process messages, handling multi-line messages
    let mut all_lines: Vec<Line> = Vec::new();
    let total_messages = display_messages.len();

    for (idx, &message) in display_messages.iter().enumerate() {
        if let Some(stripped) = message.strip_prefix("> ") {
            // Special handling for user messages with newlines
            if stripped.contains('\n') {
                // Split the message by newlines
                let lines: Vec<&str> = stripped.split('\n').collect();

                // Process the first line with the "YOU: " prefix
                all_lines.push(Line::from(vec![
                    Span::styled(
                        "YOU: ",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(lines[0], AppStyles::user_input()),
                ]));

                // Process additional lines with indentation
                for line in &lines[1..] {
                    all_lines.push(Line::from(vec![
                        Span::styled(
                            "     ", // 5 spaces to align with "YOU: "
                            Style::default()
                                .fg(Color::LightBlue)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(*line, AppStyles::user_input()),
                    ]));
                }
            } else {
                // Single-line user message
                all_lines.push(format_message(
                    message,
                    idx,
                    total_messages,
                    highlight_on,
                    app.debug_messages,
                ));
            }
        } else {
            // Regular message formatting for non-user messages
            all_lines.push(format_message(
                message,
                idx,
                total_messages,
                highlight_on,
                app.debug_messages,
            ));
        }
    }

    // Apply scrolling to show only messages that fit in the visible area
    let visible_messages = if all_lines.len() <= visible_area.height as usize {
        // If all lines fit, show them all
        all_lines
    } else {
        // Apply scrolling with bounds checking
        let max_scroll = all_lines.len().saturating_sub(visible_area.height as usize);
        let effective_scroll = app.scroll_position.min(max_scroll);

        all_lines
            .into_iter()
            .skip(effective_scroll)
            .take(visible_area.height as usize)
            .collect()
    };

    // Create a scrollable paragraph for the messages
    let has_more_above = app.scroll_position > 0;
    let has_more_below =
        app.scroll_position + (visible_area.height as usize) < display_messages.len(); // Use original message count for scroll indication

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

    // Create paragraph with the styled messages
    Paragraph::new(Text::from(visible_messages))
        .block(message_block)
        .wrap(Wrap { trim: false }) // Set trim to false to preserve message formatting
        .scroll((0, 0)) // Explicit scrolling control to prevent auto-scrolling issues
}

/// Create an input box for chat or API key input
pub fn create_input_box(app: &App, is_api_key: bool) -> Paragraph {
    // Determine placeholder text based on input context
    let placeholder = if is_api_key {
        match app.current_model().name.as_str() {
            "GPT-4o" => "Enter your OpenAI API key and press Enter...",
            _ => "Enter your Anthropic API key and press Enter...",
        }
    } else {
        "" // Empty placeholder for regular input
    };

    // Create appropriate input content based on mode
    let input_content = if app.input.is_empty() {
        // For empty input, just show the prompt and placeholder
        Text::from(Span::styled(
            format!("> {}", placeholder),
            AppStyles::hint(),
        ))
    } else if is_api_key {
        // Mask the API key with asterisks for privacy
        Text::from(Span::raw(format!("> {}", "*".repeat(app.input.len()))))
    } else if !app.input.contains('\n') {
        // Single line input - simpler case
        Text::from(vec![Line::from(vec![
            Span::raw("> "),
            Span::styled(
                app.input.as_str(),
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ])])
    } else {
        // Multiline input
        let text_style = Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD);

        // Split input into lines and process each one
        let input_str = app.input.as_str(); // Get a reference to the string
        let lines: Vec<&str> = input_str.split('\n').collect();
        let trailing_newline = input_str.ends_with('\n');

        // Convert to styled Lines
        let mut styled_lines = Vec::new();

        // Process each line
        for (idx, &line) in lines.iter().enumerate() {
            if idx == 0 {
                // First line gets the prompt
                styled_lines.push(Line::from(vec![
                    Span::raw("> "),
                    Span::styled(line, text_style),
                ]));
            } else {
                // Subsequent lines get indentation
                styled_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line, text_style),
                ]));
            }
        }

        // If the input ends with a newline, add an empty line with indentation
        if trailing_newline {
            styled_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("", text_style),
            ]));
        }

        Text::from(styled_lines)
    };

    // Create the title based on context
    let title = if is_api_key {
        "API Key"
    } else {
        "Input (Type / for commands)"
    };

    Paragraph::new(input_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false }) // Don't trim to preserve proper newline formatting
}

/// Create a command menu list for selection
pub fn create_command_menu(app: &App) -> List {
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
                // Highlight the selected command with an arrow indicator and blue text
                ListItem::new(format!("▶ {} - {}", cmd.name, cmd.description))
                    .style(AppStyles::command_highlight())
            } else {
                // Non-selected commands with proper spacing
                ListItem::new(format!("  {} - {}", cmd.name, cmd.description))
                    .style(Style::default().fg(Color::Gray))
            }
        })
        .collect();

    // Create the list with a subtle style
    List::new(command_items)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::Gray)) // Default text color
        .highlight_style(AppStyles::command_highlight()) // Use the same style for consistency
}

/// Create a list of models for selection in setup mode
pub fn create_model_list(app: &App) -> List {
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
                ListItem::new(format!("→ {}", content)).style(AppStyles::highlight())
            } else {
                ListItem::new(format!("  {}", content))
            }
        })
        .collect();

    List::new(models)
        .block(Block::default().borders(Borders::ALL).title("Models"))
        .highlight_style(AppStyles::highlight())
}

/// Create a progress display for model downloads
pub fn create_progress_display(app: &App) -> Paragraph {
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

    Paragraph::new(progress_text)
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .style(AppStyles::success())
}

/// Create an information display for API key setup
pub fn create_api_key_info(app: &App) -> List {
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
        .block(Block::default().borders(Borders::ALL).title("Information"))
        .style(Style::default().fg(Color::Yellow))
}

/// Create a permission dialog
pub fn create_permission_dialog(_app: &App, _area: Rect) -> Block {
    Block::default()
        .title("Permission Required")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
}

/// Create permission dialog content
pub fn create_permission_content(app: &App) -> Paragraph {
    let tool = app.pending_tool.as_ref().unwrap();
    let description = tool.description.to_string();

    let info_text = Text::from(vec![
        Line::from(vec![
            Span::styled("⚠️  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Permission Required",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Tool: "),
            Span::styled(&tool.tool_name, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Action: "),
            Span::styled(description, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Y to allow or N to deny",
            Style::default().fg(Color::Gray),
        )]),
    ]);

    Paragraph::new(info_text)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
}

/// Format a message based on its type and content
/// Returns the styled line for the message
fn format_message(
    message: &str,
    idx: usize,
    total_messages: usize,
    highlight_on: bool,
    debug_enabled: bool,
) -> Line {
    // Check if this is the last message and should be highlighted
    let is_newest_msg = idx == total_messages - 1;

    // Handle ANSI colorized messages with tool indicators first
    if message.contains("\x1b[32m⏺\x1b[0m") || message.contains("\x1b[31m⏺\x1b[0m") {
        // Use appropriate formatter based on message type
        if message.contains("All tools executed successfully") {
            return format_success_message(message);
        } else {
            return format_tool_message(message, is_newest_msg, highlight_on);
        }
    }

    // Handle messages with the direct ⏺ indicator
    if message.starts_with("⏺ ") {
        // Use appropriate formatter based on message type/context
        if message.contains("All tools executed successfully") {
            return format_success_message(message);
        } else {
            return format_tool_message(message, is_newest_msg, highlight_on);
        }
    }

    // Process other message types
    if message.starts_with("DEBUG:") {
        // Only show debug messages in debug mode
        if debug_enabled {
            Line::from(vec![Span::styled(
                message,
                Style::default().fg(Color::Yellow),
            )])
        } else {
            Line::from("")
        }
    } else if let Some(stripped) = message.strip_prefix("> ") {
        // User messages - cyan
        Line::from(vec![
            Span::styled(
                "YOU: ",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(stripped, AppStyles::user_input()),
        ])
    } else if message.starts_with("Error:") || message.starts_with("ERROR:") {
        // Error messages - red
        Line::from(vec![Span::styled(message, AppStyles::error())])
    } else if message.starts_with("Status:") {
        // Status messages - blue
        Line::from(vec![Span::styled(
            message,
            Style::default().fg(Color::Blue),
        )])
    } else if message.starts_with("★") {
        // Title/welcome messages - light cyan with bold
        Line::from(vec![Span::styled(message, AppStyles::title())])
    } else if message == "Thinking..." {
        // Legacy thinking message
        Line::from(vec![
            Span::styled("⏺ ", Style::default().fg(Color::Yellow)),
            Span::styled("Thinking...", AppStyles::thinking()),
        ])
    } else if message.starts_with("[thinking]") {
        // AI Thinking/reasoning message
        format_thinking_message(message, is_newest_msg, highlight_on)
    } else if message.starts_with("[tool] ") {
        // Tool execution message
        format_tool_message(message, is_newest_msg, highlight_on)
    } else if message.starts_with("[success] ") {
        // Success/completion message
        format_success_message(message)
    } else if message.starts_with("[wait]") {
        // Progress/wait message with white circle
        format_wait_message(message)
    } else if message.starts_with("[error] ") {
        // Error/failure message
        format_error_message(message)
    } else {
        // Model responses or other text
        format_model_response(message)
    }
}

// Helper functions for formatting various message types
fn format_thinking_message(message: &str, is_newest_msg: bool, highlight_on: bool) -> Line {
    let thinking_content = message.strip_prefix("[thinking] ").unwrap_or(message);

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
        Line::from(vec![Span::styled(message, style)])
    } else {
        // Legacy format without icon - add the circle
        Line::from(vec![
            Span::styled(
                "⏺ ",
                if is_newest_msg && highlight_on {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            ),
            Span::styled(thinking_content, style),
        ])
    }
}

fn format_tool_message(message: &str, _is_newest_msg: bool, _highlight_on: bool) -> Line {
    // Handle ANSI colorized messages with proper spacing
    if message.contains("\x1b[32m⏺\x1b[0m") {
        // Find where the actual message content starts (right after the ANSI sequence)
        if let Some(ansi_end_pos) = message.find("\x1b[0m") {
            // Find where the content starts (after the ANSI sequences and a space)
            let content_start = ansi_end_pos + 4; // 4 is length of "\x1b[0m"
            if content_start < message.len() {
                let content = &message[content_start..];
                // Create a line with proper ratatui styling
                return Line::from(vec![
                    Span::styled("⏺ ", Style::default().fg(Color::Green)),
                    Span::styled(
                        content.trim_start(),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
            }
        }
    }

    // Handle normal tool messages
    let tool_content = message.strip_prefix("[tool] ").unwrap_or(message);

    // Apply standard tool styling
    let style = Style::default()
        .fg(Color::LightBlue)
        .add_modifier(Modifier::BOLD);

    if tool_content.starts_with("⏺ ") {
        // Has a circle already - extract content and add proper space
        let pure_content = tool_content
            .strip_prefix("⏺ ")
            .unwrap_or(tool_content)
            .trim_start();
        Line::from(vec![
            Span::styled("⏺ ", Style::default().fg(Color::Green)),
            Span::styled(pure_content, style),
        ])
    } else {
        // Default format with green indicator
        Line::from(vec![
            Span::styled("⏺ ", Style::default().fg(Color::Green)),
            Span::styled(tool_content, style),
        ])
    }
}

fn format_success_message(message: &str) -> Line {
    // Handle ANSI colorized messages with proper spacing
    if message.contains("\x1b[32m⏺\x1b[0m") {
        // Find where the actual message content starts (right after the ANSI sequence)
        if let Some(ansi_end_pos) = message.find("\x1b[0m") {
            // Find where the content starts (after the ANSI sequences and a space)
            let content_start = ansi_end_pos + 4; // 4 is length of "\x1b[0m"
            if content_start < message.len() {
                let content = &message[content_start..];
                // Create a line with proper ratatui styling
                return Line::from(vec![
                    Span::styled("⏺ ", Style::default().fg(Color::Green)),
                    Span::styled(content.trim_start(), Style::default().fg(Color::Green)),
                ]);
            }
        }
    }

    let content = message.strip_prefix("[success] ").unwrap_or(message);

    // Handle direct messages with tool circle already (like "⏺ All tools executed successfully")
    if content.starts_with("⏺ ") {
        let pure_content = content.strip_prefix("⏺ ").unwrap_or(content).trim_start();
        return Line::from(vec![
            Span::styled("⏺ ", Style::default().fg(Color::Green)),
            Span::styled(pure_content, Style::default().fg(Color::Green)),
        ]);
    }

    // Default success message format
    Line::from(vec![
        Span::styled("⏺ ", Style::default().fg(Color::Green)),
        Span::styled(content, Style::default().fg(Color::Green)),
    ])
}

fn format_wait_message(message: &str) -> Line {
    let wait_content = message.strip_prefix("[wait] ").unwrap_or(message);

    if wait_content.starts_with("⚪ ") {
        // New format with white circle emoji
        // Extract content and format consistently
        let content = wait_content.strip_prefix("⚪ ").unwrap_or(wait_content);
        Line::from(vec![
            Span::styled("⚪ ", Style::default().fg(Color::LightYellow)),
            Span::styled(content, Style::default().fg(Color::Yellow)),
        ])
    } else {
        // Legacy format needs an icon
        Line::from(vec![
            Span::styled("⚪ ", Style::default().fg(Color::LightYellow)),
            Span::styled(wait_content, Style::default().fg(Color::Yellow)),
        ])
    }
}

fn format_error_message(message: &str) -> Line {
    // Handle ANSI colorized messages (red circle)
    if message.contains("\x1b[31m⏺\x1b[0m") {
        // Find where the actual message content starts (right after the ANSI sequence)
        if let Some(ansi_end_pos) = message.find("\x1b[0m") {
            // Find where the content starts (after the ANSI sequences and a space)
            let content_start = ansi_end_pos + 4; // 4 is length of "\x1b[0m"
            if content_start < message.len() {
                let content = &message[content_start..];
                // Create a line with properly styled red circle
                return Line::from(vec![
                    Span::styled("⏺ ", AppStyles::error()),
                    Span::styled(content.trim_start(), AppStyles::error()),
                ]);
            }
        }
    }

    let error_content = message.strip_prefix("[error] ").unwrap_or(message);

    // Standard error formatting with red indicator
    Line::from(vec![
        Span::styled("⏺ ", AppStyles::error()),
        Span::styled(error_content, AppStyles::error()),
    ])
}

fn format_model_response(message: &str) -> Line {
    // Check various special cases for model responses
    if message == "Final response:" {
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
    } else if message.trim().is_empty() {
        Line::from("")
    } else {
        // Default case - assume this is a model response
        // Just display the message directly without adding "Final answer:" prefix
        Line::from(vec![Span::raw(message)])
    }
}

/// Create a shortcuts panel for display below the input box
pub fn create_shortcuts_panel(app: &App) -> Paragraph {
    // Only show shortcuts when the input is empty
    if app.input.is_empty() {
        if app.show_detailed_shortcuts {
            // Show detailed shortcuts when ? has been pressed and input is empty
            let shortcuts_text = Text::from(vec![
                Line::from(vec![Span::styled(
                    "Keyboard Shortcuts",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled(
                        "/ ",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("Show commands menu"),
                ]),
                Line::from(vec![
                    Span::styled(
                        "\\⏎ ",
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("Add newline in input"),
                ]),
            ]);

            Paragraph::new(shortcuts_text).style(Style::default().fg(Color::Gray))
        } else if app.show_shortcuts_hint {
            // Show the basic hint only when input is empty
            let shortcuts_text = Text::from(vec![Line::from(vec![
                Span::styled(
                    "? ",
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("for shortcuts", Style::default().fg(Color::DarkGray)),
            ])]);

            Paragraph::new(shortcuts_text).style(Style::default().fg(Color::Gray))
        } else {
            // Empty placeholder
            Paragraph::new("")
        }
    } else {
        // Hide shortcuts when anything is typed in the input
        Paragraph::new("")
    }
}
