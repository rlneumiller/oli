use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    IoError(std::io::Error),
    NetworkError(String),
    #[allow(dead_code)]
    ModelError(String),
    #[allow(dead_code)]
    ValidationError(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::IoError(e) => write!(f, "IO Error: {}", e),
            AppError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            AppError::ModelError(msg) => write!(f, "Model Error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            AppError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::NetworkError(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Other(err.to_string())
    }
}
