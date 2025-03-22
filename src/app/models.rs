use crate::models::ModelConfig;
use anyhow::Result;
use std::path::Path;
use std::sync::mpsc;

// Re-export from src/app/permissions.rs
pub use super::permissions::ToolPermissionStatus;

pub trait ModelManager {
    fn current_model(&self) -> &ModelConfig;
    fn select_next_model(&mut self);
    fn select_prev_model(&mut self);
    fn load_model(&mut self, model_path: &Path) -> Result<()>;
    fn setup_models(&mut self, tx: mpsc::Sender<String>) -> Result<()>;
    fn get_agent_model(&self) -> Option<String>;
}
