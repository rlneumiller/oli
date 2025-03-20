use ratatui::style::{Color, Modifier, Style};

/// UI styles used throughout the application with a cleaner, more modern palette
pub struct AppStyles;

impl AppStyles {
    /// Primary color for the application
    pub fn primary_color() -> Color {
        Color::Rgb(86, 182, 194) // Soft teal
    }

    /// Secondary color for the application
    pub fn secondary_color() -> Color {
        Color::Rgb(240, 240, 240) // Almost white
    }

    /// Accent color for highlights and important elements
    pub fn accent_color() -> Color {
        Color::Rgb(95, 129, 157) // Soft blue
    }

    /// Background color for selected elements
    pub fn selection_bg() -> Color {
        Color::Rgb(45, 45, 45) // Dark gray
    }

    /// Style for title text
    pub fn title() -> Style {
        Style::default()
            .fg(Self::primary_color())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for highlighted text
    pub fn highlight() -> Style {
        Style::default()
            .fg(Self::accent_color())
            .bg(Self::selection_bg())
    }

    /// Style for command highlight
    pub fn command_highlight() -> Style {
        Style::default()
            .fg(Self::accent_color())
            .add_modifier(Modifier::BOLD)
    }

    /// Style for error messages
    pub fn error() -> Style {
        Style::default().fg(Color::Rgb(225, 95, 95)) // Softer red
    }

    /// Style for success messages
    pub fn success() -> Style {
        Style::default().fg(Color::Rgb(142, 192, 124)) // Softer green
    }

    /// Style for user input/messages
    pub fn user_input() -> Style {
        Style::default().fg(Self::primary_color())
    }

    /// Style for status bar
    pub fn status_bar() -> Style {
        Style::default().fg(Color::Black).bg(Self::accent_color())
    }

    /// Style for thinking/processing state
    pub fn thinking() -> Style {
        Style::default()
            .fg(Color::Rgb(240, 180, 100)) // Soft amber
            .add_modifier(Modifier::ITALIC)
    }

    /// Style for hints and subtle text
    pub fn hint() -> Style {
        Style::default().fg(Color::Rgb(150, 150, 150)) // Medium gray
    }

    /// Style for the text cursor
    #[allow(dead_code)]
    pub fn cursor() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::primary_color())
            .add_modifier(Modifier::BOLD)
    }

    /// Border style for panels
    pub fn border() -> Style {
        Style::default().fg(Color::Rgb(100, 100, 100)) // Subtle border
    }

    /// Style for section headers
    pub fn section_header() -> Style {
        Style::default()
            .fg(Self::accent_color())
            .add_modifier(Modifier::BOLD)
    }
}
