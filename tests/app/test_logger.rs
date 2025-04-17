use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use chrono::Local;
use oli_server::app::{format_log, format_log_with_color, LogLevel, Logger};

// Mock implementation of Logger for testing
struct MockLogger {
    log_directory: PathBuf,
    log_file_path: PathBuf,
    show_logs: bool,
}

impl MockLogger {
    fn new(log_directory: PathBuf, log_file_path: PathBuf) -> Self {
        Self {
            log_directory,
            log_file_path,
            show_logs: false,
        }
    }
}

impl Logger for MockLogger {
    fn log(&mut self, _message: &str, _args: &[&str]) {
        // Mock implementation - doesn't need to do anything for tests
    }

    fn toggle_log_view(&mut self) {
        self.show_logs = !self.show_logs;
    }

    fn get_log_directory(&self) -> PathBuf {
        self.log_directory.clone()
    }

    fn get_log_file_path(&self) -> PathBuf {
        self.log_file_path.clone()
    }

    fn write_log_to_file(&self, message: &str) -> anyhow::Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = self.log_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Append the message to the log file
        fs::write(&self.log_file_path, message)?;
        Ok(())
    }
}

#[test]
fn test_log_level_as_str() {
    assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
    assert_eq!(LogLevel::Info.as_str(), "INFO");
    assert_eq!(LogLevel::Warning.as_str(), "WARN");
    assert_eq!(LogLevel::Error.as_str(), "ERROR");
}

#[test]
fn test_log_level_color_code() {
    assert_eq!(LogLevel::Debug.color_code(), "\x1b[36m"); // Cyan
    assert_eq!(LogLevel::Info.color_code(), "\x1b[32m"); // Green
    assert_eq!(LogLevel::Warning.color_code(), "\x1b[33m"); // Yellow
    assert_eq!(LogLevel::Error.color_code(), "\x1b[31m"); // Red
}

#[test]
fn test_format_log() {
    let test_message = "Test log message";
    let formatted = format_log(LogLevel::Info, test_message);

    // Test that formatting contains expected parts
    assert!(formatted.contains("[INFO]"));
    assert!(formatted.contains(test_message));

    // Check date format (this is a less strict test since the timestamp will vary)
    let now = Local::now();
    let year = now.format("%Y").to_string();
    assert!(formatted.contains(&year));
}

#[test]
fn test_format_log_with_color() {
    let test_message = "Test colored log message";
    let formatted = format_log_with_color(LogLevel::Error, test_message);

    // Test that formatting contains expected parts
    assert!(formatted.contains(LogLevel::Error.color_code()));
    assert!(formatted.contains(LogLevel::Error.as_str()));
    assert!(formatted.contains(test_message));
    assert!(formatted.contains("\x1b[0m")); // Reset color code
}

#[test]
fn test_logger_toggle_log_view() {
    let temp_dir = tempdir().unwrap();
    let log_dir = temp_dir.path().to_path_buf();
    let log_file = log_dir.join("test.log");

    let mut logger = MockLogger::new(log_dir, log_file);
    assert!(!logger.show_logs);

    logger.toggle_log_view();
    assert!(logger.show_logs);

    logger.toggle_log_view();
    assert!(!logger.show_logs);
}

#[test]
fn test_logger_get_paths() {
    let temp_dir = tempdir().unwrap();
    let log_dir = temp_dir.path().to_path_buf();
    let log_file = log_dir.join("test.log");

    let logger = MockLogger::new(log_dir.clone(), log_file.clone());

    assert_eq!(logger.get_log_directory(), log_dir);
    assert_eq!(logger.get_log_file_path(), log_file);
}

#[test]
fn test_write_log_to_file() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let log_dir = temp_dir.path().to_path_buf();
    let log_file = log_dir.join("test.log");

    let logger = MockLogger::new(log_dir, log_file.clone());
    let test_message = "Test log to file";

    logger.write_log_to_file(test_message)?;

    // Verify file contents
    let file_contents = fs::read_to_string(&log_file)?;
    assert_eq!(file_contents, test_message);

    Ok(())
}
