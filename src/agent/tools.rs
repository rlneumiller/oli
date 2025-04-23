use crate::tools::{fs::file_ops::FileOps, fs::search::SearchTools};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolType {
    FileReadTool,
    GlobTool,
    GrepTool,
    LS,
    Edit,
    Replace,
    Bash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadToolParams {
    pub file_path: String,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobToolParams {
    pub pattern: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepToolParams {
    pub pattern: String,
    pub include: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSParams {
    pub path: String,
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditParams {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceParams {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashParams {
    pub command: String,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "params")]
pub enum ToolCall {
    FileReadTool(FileReadToolParams),
    GlobTool(GlobToolParams),
    GrepTool(GrepToolParams),
    LS(LSParams),
    Edit(EditParams),
    Replace(ReplaceParams),
    Bash(BashParams),
}

impl ToolCall {
    pub fn execute(&self) -> Result<String> {
        match self {
            ToolCall::FileReadTool(params) => {
                // Get the global RPC server to send notification
                if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
                    // Generate a unique ID for this execution
                    let tool_id = format!(
                        "fileread-direct-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    );

                    // First, send a "started" notification
                    let start_notification = serde_json::json!({
                        "type": "started",
                        "execution": {
                            "id": tool_id,
                            "task_id": "direct-task",
                            "name": "Read",
                            "status": "running",
                            "startTime": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis(),
                            "message": format!("Reading file: {}", params.file_path),
                            "metadata": {
                                "file_path": params.file_path,
                                "description": format!("Reading file: {}", params.file_path),
                            }
                        }
                    });

                    // Send start notification
                    rpc_server
                        .send_notification("tool_status", start_notification)
                        .ok();

                    // Add a brief delay to ensure the running state is visible
                    // This simulates a longer-running tool operation
                    std::thread::sleep(std::time::Duration::from_millis(1000));

                    // Read the file
                    let path = PathBuf::from(&params.file_path);
                    // Always use read_file_lines with provided offset and limit
                    let result = FileOps::read_file_lines(&path, params.offset, Some(params.limit));

                    // For successful reads, send a completion notification
                    if let Ok(ref content) = result {
                        // Count the number of lines
                        let line_count = content.lines().count();

                        // Send completion notification
                        let complete_notification = serde_json::json!({
                            "type": "updated",
                            "execution": {
                                "id": tool_id,
                                "task_id": "direct-task",
                                "name": "Read",
                                "status": "success",
                                "startTime": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() - 1000, // 1 second ago
                                "endTime": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis(),
                                "message": format!("Read {} lines from file", line_count),
                                "metadata": {
                                    "file_path": params.file_path,
                                    "lines": line_count,
                                    "description": format!("Read {} lines from file", line_count),
                                }
                            }
                        });

                        // Send completion notification
                        rpc_server
                            .send_notification("tool_status", complete_notification)
                            .ok();
                    } else if let Err(ref e) = result {
                        // Send error notification for failed reads
                        let error_notification = serde_json::json!({
                            "type": "updated",
                            "execution": {
                                "id": tool_id,
                                "task_id": "direct-task",
                                "name": "Read",
                                "status": "error",
                                "startTime": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() - 1000, // 1 second ago
                                "endTime": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis(),
                                "message": format!("Error reading file: {}", e),
                                "metadata": {
                                    "file_path": params.file_path,
                                    "description": format!("Error reading file: {}", e),
                                }
                            }
                        });

                        // Send error notification
                        rpc_server
                            .send_notification("tool_status", error_notification)
                            .ok();
                    }

                    result
                } else {
                    // No RPC server available, just read the file
                    let path = PathBuf::from(&params.file_path);
                    // Always use read_file_lines with provided offset and limit
                    FileOps::read_file_lines(&path, params.offset, Some(params.limit))
                }
            }
            ToolCall::GlobTool(params) => {
                let results = if let Some(path) = &params.path {
                    let dir_path = PathBuf::from(path);
                    SearchTools::glob_search_in_dir(&dir_path, &params.pattern)?
                } else {
                    SearchTools::glob_search(&params.pattern)?
                };

                let mut output = format!(
                    "Found {} files matching pattern '{}':\n\n",
                    results.len(),
                    params.pattern
                );
                for (i, path) in results.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, path.display()));
                }
                Ok(output)
            }
            ToolCall::GrepTool(params) => {
                let search_dir = params.path.as_ref().map(Path::new);
                let results = SearchTools::grep_search(
                    &params.pattern,
                    params.include.as_deref(),
                    search_dir,
                )?;

                let mut output = format!(
                    "Found {} matches for pattern '{}':\n\n",
                    results.len(),
                    params.pattern
                );
                for (path, line_num, line) in results {
                    output.push_str(&format!("{}:{}:{}\n", path.display(), line_num, line));
                }
                Ok(output)
            }
            ToolCall::LS(params) => {
                let path = PathBuf::from(&params.path);
                let entries = FileOps::list_directory(&path)?;

                let mut output = format!("Directory listing for '{}':\n", params.path);
                for (i, entry) in entries.iter().enumerate() {
                    let file_type = if entry.is_dir() { "DIR" } else { "FILE" };
                    output.push_str(&format!(
                        "{:3}. [{}] {}\n",
                        i + 1,
                        file_type,
                        entry.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                Ok(output)
            }
            ToolCall::Edit(params) => {
                let path = PathBuf::from(&params.file_path);
                let diff = FileOps::edit_file(&path, &params.old_string, &params.new_string)?;
                Ok(diff)
            }
            ToolCall::Replace(params) => {
                let path = PathBuf::from(&params.file_path);
                let diff = FileOps::write_file_with_diff(&path, &params.content)?;
                Ok(diff)
            }
            ToolCall::Bash(params) => {
                use std::process::{Command, Stdio};

                // Use a simpler execution model to avoid issues with wait_timeout and async
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(&params.command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let result = if output.status.success() {
                    stdout
                } else {
                    format!(
                        "Command failed with exit code: {}\nStdout: {}\nStderr: {}",
                        output.status.code().unwrap_or(-1),
                        stdout,
                        stderr
                    )
                };

                Ok(result)
            }
        }
    }
}

pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        serde_json::json!({
            "name": "FileReadTool",
            "description": "Reads a file from the local filesystem. The file_path must be an absolute path.",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The line number to start reading from (required, 0-based)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The number of lines to read (required)"
                    }
                },
                "required": ["file_path", "offset", "limit"]
            }
        }),
        serde_json::json!({
            "name": "GlobTool",
            "description": "Fast file pattern matching tool using glob patterns like '**/*.rs'",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files against"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in (optional)"
                    }
                },
                "required": ["pattern"]
            }
        }),
        serde_json::json!({
            "name": "GrepTool",
            "description": "Fast content search tool using regular expressions",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regular expression pattern to search for in file contents"
                    },
                    "include": {
                        "type": "string",
                        "description": "File pattern to include in the search (e.g. \"*.rs\", \"*.{rs,toml}\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in (optional)"
                    }
                },
                "required": ["pattern"]
            }
        }),
        serde_json::json!({
            "name": "LS",
            "description": "Lists files and directories in a given path",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the directory to list"
                    },
                    "ignore": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "List of glob patterns to ignore (optional)"
                    }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "name": "Edit",
            "description": "Edits a file by replacing one string with another",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to modify"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "The text to replace (must be unique within the file)"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "The text to replace it with"
                    }
                },
                "required": ["file_path", "old_string", "new_string"]
            }
        }),
        serde_json::json!({
            "name": "Replace",
            "description": "Completely replaces a file with new content",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }
        }),
        serde_json::json!({
            "name": "Bash",
            "description": "Executes a bash command",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to execute"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Optional timeout in milliseconds (max 600000)"
                    }
                },
                "required": ["command"]
            }
        }),
    ]
}
