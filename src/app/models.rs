use serde::{Deserialize, Serialize};

/// Tool permission status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolPermissionStatus {
    Pending,
    Granted,
    Denied,
}
