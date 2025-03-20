use crate::agent::core::Agent;
use crate::app::commands::SpecialCommand;
use crate::app::models::ToolPermissionStatus;
use crate::app::permissions::PendingToolExecution;
use crate::models::ModelConfig;
use std::sync::mpsc;
use std::time::Instant;
use tokio::runtime::Runtime;

#[derive(Debug, PartialEq)]
pub enum AppState {
    Setup,
    ApiKeyInput,
    Error(String),
    Chat,
}

use tui_textarea::TextArea;

pub struct App {
    pub state: AppState,
    pub textarea: TextArea<'static>, // TextArea widget for improved multiline input
    pub input: String,               // Keep for backward compatibility during transition
    pub messages: Vec<String>,
    pub download_progress: Option<(u64, u64)>,
    pub selected_model: usize,
    pub available_models: Vec<ModelConfig>,
    pub download_active: bool,
    pub error_message: Option<String>,
    pub debug_messages: bool,
    pub scroll_position: usize,
    pub last_query_time: Instant,
    pub last_message_time: Instant, // Timestamp for message animations
    pub use_agent: bool,
    pub agent: Option<Agent>,
    pub tokio_runtime: Option<Runtime>,
    pub agent_progress_rx: Option<mpsc::Receiver<String>>,
    pub api_key: Option<String>,
    pub current_working_dir: Option<String>,
    // Command-related fields
    pub command_mode: bool,
    pub available_commands: Vec<SpecialCommand>,
    pub selected_command: usize,
    pub show_command_menu: bool,
    // Tool permission-related fields
    pub permission_required: bool, // If true, we're waiting for user input on a tool permission
    pub pending_tool: Option<PendingToolExecution>, // The tool waiting for permission
    pub tool_permission_status: ToolPermissionStatus, // Current permission status
    pub tool_execution_in_progress: bool, // Flag to indicate active tool execution
    pub show_intermediate_steps: bool, // Show intermediate steps like tool use and file reads
    pub show_shortcuts_hint: bool, // Show the shortcut hint below input box
    pub show_detailed_shortcuts: bool, // Show all shortcuts when ? is pressed
    // Cursor position in input - kept for backward compatibility
    pub cursor_position: usize, // Current cursor position in the input string
}
