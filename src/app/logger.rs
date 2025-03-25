use anyhow::Result;
use std::path::PathBuf;

/// Logger trait for writing logs to a file and displaying them in the TUI
pub trait Logger {
    /// Log a message with optional formatting arguments
    fn log(&mut self, message: &str, args: &[&str]);

    /// Toggle between showing logs and normal output
    fn toggle_log_view(&mut self);

    /// Get the log directory path
    fn get_log_directory(&self) -> PathBuf;

    /// Get the log file path for the current session
    fn get_log_file_path(&self) -> PathBuf;

    /// Write a log message to file
    fn write_log_to_file(&self, message: &str) -> Result<()>;
}

/// Log level for messages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    /// Debug level for detailed information
    Debug,
    /// Info level for general information
    Info,
    /// Warning level for potential issues
    Warning,
    /// Error level for error conditions
    Error,
}

impl LogLevel {
    /// Get a string representation of the log level
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Get a color code for the log level
    pub fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Debug => "\x1b[36m",   // Cyan
            LogLevel::Info => "\x1b[32m",    // Green
            LogLevel::Warning => "\x1b[33m", // Yellow
            LogLevel::Error => "\x1b[31m",   // Red
        }
    }
}

/// Format a log message with level, timestamp, and message
pub fn format_log(level: LogLevel, message: &str) -> String {
    let now = chrono::Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");

    format!("[{}] [{}] {}", timestamp, level.as_str(), message)
}

/// Format a log message with color for terminal display
pub fn format_log_with_color(level: LogLevel, message: &str) -> String {
    let now = chrono::Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");
    let reset = "\x1b[0m";

    format!(
        "[{}] [{}{}{}] {}",
        timestamp,
        level.color_code(),
        level.as_str(),
        reset,
        message
    )
}
