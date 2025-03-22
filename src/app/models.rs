use crate::models::ModelConfig;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

// Re-export from src/app/permissions.rs
pub use super::permissions::ToolPermissionStatus;

pub trait ModelManager {
    fn current_model(&self) -> &ModelConfig;
    fn select_next_model(&mut self);
    fn select_prev_model(&mut self);
    fn models_dir() -> Result<PathBuf>;
    fn model_path(&self, model_name: &str) -> Result<PathBuf>;
    fn verify_model(&self, path: &Path) -> Result<()>;
    fn verify_static(path: &Path) -> Result<()>;
    fn load_model(&mut self, model_path: &Path) -> Result<()>;
    fn setup_models(&mut self, tx: mpsc::Sender<String>) -> Result<()>;
    fn download_model_with_path(&mut self, tx: mpsc::Sender<String>, path: &Path) -> Result<()>;
    fn download_file(&mut self, path: &Path, tx: mpsc::Sender<String>) -> Result<()>;
    fn get_agent_model(&self) -> Option<String>;
    fn attempt_download(url: &str, path: &Path, tx: &mpsc::Sender<String>) -> Result<(), String>;
}
