use ratatui::style::{Color, Modifier, Style};

/// UI styles used throughout the application
pub struct AppStyles;

impl AppStyles {
    /// Style for title text
    pub fn title() -> Style {
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for highlighted text
    pub fn highlight() -> Style {
        Style::default().fg(Color::LightBlue)
    }

    /// Style for command highlight
    pub fn command_highlight() -> Style {
        Style::default()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for error messages
    pub fn error() -> Style {
        Style::default().fg(Color::Red)
    }

    /// Style for success messages
    pub fn success() -> Style {
        Style::default().fg(Color::Green)
    }

    /// Style for user input/messages
    pub fn user_input() -> Style {
        Style::default().fg(Color::Cyan)
    }

    /// Style for status bar
    pub fn status_bar() -> Style {
        Style::default().fg(Color::Black).bg(Color::LightBlue)
    }

    /// Style for thinking/processing state
    pub fn thinking() -> Style {
        Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::ITALIC)
    }

    /// Style for hints and subtle text
    pub fn hint() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    /// Style for the text cursor
    pub fn cursor() -> Style {
        Style::default()
            .fg(Color::Black) // Black text
            .bg(Color::LightCyan) // Light cyan background
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED) // Reversed colors for high visibility
    }
}
