// Tool permission handling

#[derive(Debug, Clone, PartialEq)]
pub enum ToolPermissionStatus {
    Pending, // Waiting for user confirmation
    Granted, // User has granted permission for this tool
    Denied,  // User has denied permission for this tool
}

// Pending tool execution that needs confirmation
#[derive(Debug, Clone)]
pub struct PendingToolExecution {
    pub tool_name: String,
    pub tool_args: String,
    pub description: String,
}

pub trait PermissionHandler {
    fn requires_permission(&self, tool_name: &str) -> bool;
    fn request_tool_permission(&mut self, tool_name: &str, args: &str) -> ToolPermissionStatus;
    fn handle_permission_response(&mut self, granted: bool);
    fn extract_argument(&self, args: &str, arg_name: &str) -> Option<String>;
    fn requires_permission_check(&self) -> bool;
}
