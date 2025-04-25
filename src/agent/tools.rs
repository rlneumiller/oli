use crate::tools::{
    fs::file_ops::FileOps,
    fs::search::SearchTools,
    lsp::{
        DefinitionParams, LspServerManager, ModelsCodeLensParams as CodeLensParams,
        ModelsDocumentSymbolParams as DocumentSymbolParams,
        ModelsSemanticTokensParams as SemanticTokensParams,
    },
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolType {
    FileReadTool,
    GlobTool,
    GrepTool,
    LSTool,
    Edit,
    Replace,
    Bash,
    DocumentSymbol,
    SemanticTokens,
    CodeLens,
    Definition,
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
pub struct LSToolParams {
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
    LSTool(LSToolParams),
    Edit(EditParams),
    Replace(ReplaceParams),
    Bash(BashParams),
    DocumentSymbol(DocumentSymbolParams),
    SemanticTokens(SemanticTokensParams),
    CodeLens(CodeLensParams),
    Definition(DefinitionParams),
}

// Helper function to create and send tool status notifications
fn send_tool_notification(
    tool_name: &str,
    status: &str,
    message: &str,
    metadata: serde_json::Value,
    tool_id: &str,
    start_time: u128,
) -> Result<()> {
    if let Some(rpc_server) = crate::communication::rpc::get_global_rpc_server() {
        let notification_type = if status == "running" {
            "started"
        } else {
            "updated"
        };

        let mut notification = serde_json::json!({
            "type": notification_type,
            "execution": {
                "id": tool_id,
                "task_id": "direct-task",
                "name": tool_name,
                "status": status,
                "message": message,
                "metadata": metadata
            }
        });

        // Add timestamps based on status
        if status == "running" {
            notification["execution"]["startTime"] = serde_json::json!(start_time);
        } else {
            // For success or error states, we need both start and end times
            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            notification["execution"]["startTime"] = serde_json::json!(start_time);
            notification["execution"]["endTime"] = serde_json::json!(end_time);
        }

        // Send notification
        rpc_server
            .send_notification("tool_status", notification)
            .ok();
        Ok(())
    } else {
        Ok(()) // No RPC server available, so silently succeed
    }
}

impl ToolCall {
    pub fn execute(&self) -> Result<String> {
        match self {
            ToolCall::FileReadTool(params) => {
                // Generate a unique ID for this execution
                let tool_id = format!(
                    "fileread-direct-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );

                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();

                // Send start notification
                let metadata = serde_json::json!({
                    "file_path": params.file_path,
                    "description": format!("Reading file: {}", params.file_path),
                });
                send_tool_notification(
                    "Read",
                    "running",
                    &format!("Reading file: {}", params.file_path),
                    metadata,
                    &tool_id,
                    start_time,
                )
                .ok();

                // Add a brief delay to ensure the running state is visible
                std::thread::sleep(std::time::Duration::from_millis(1000));

                // Read the file
                let path = PathBuf::from(&params.file_path);
                // Always use read_file_lines with provided offset and limit
                let result = FileOps::read_file_lines(&path, params.offset, Some(params.limit));

                // Send appropriate completion notification
                if let Ok(ref content) = result {
                    // Count the number of lines
                    let line_count = content.lines().count();

                    // Send success notification
                    let metadata = serde_json::json!({
                        "file_path": params.file_path,
                        "lines": line_count,
                        "description": format!("Read {} lines from file", line_count),
                    });
                    send_tool_notification(
                        "Read",
                        "success",
                        &format!("Read {} lines from file", line_count),
                        metadata,
                        &tool_id,
                        start_time,
                    )
                    .ok();
                } else if let Err(ref e) = result {
                    // Send error notification
                    let metadata = serde_json::json!({
                        "file_path": params.file_path,
                        "description": format!("Error reading file: {}", e),
                    });
                    send_tool_notification(
                        "Read",
                        "error",
                        &format!("Error reading file: {}", e),
                        metadata,
                        &tool_id,
                        start_time,
                    )
                    .ok();
                }

                result
            }
            ToolCall::GlobTool(params) => {
                // Generate a unique ID for this execution
                let tool_id = format!(
                    "glob-direct-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );

                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();

                // Send start notification with the pattern format and optional path
                let metadata = if let Some(path) = &params.path {
                    serde_json::json!({
                        "pattern": params.pattern,
                        "path": path,
                        "description": format!("Search(pattern: \"{}\", path: \"{}\")", params.pattern, path),
                    })
                } else {
                    serde_json::json!({
                        "pattern": params.pattern,
                        "description": format!("Search(pattern: \"{}\")", params.pattern),
                    })
                };
                let message = if let Some(path) = &params.path {
                    format!("Searching in {} with pattern: {}", path, params.pattern)
                } else {
                    format!("Searching with pattern: {}", params.pattern)
                };

                send_tool_notification(
                    "Search", "running", &message, metadata, &tool_id, start_time,
                )
                .ok();

                // Add a brief delay to ensure the running state is visible
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Perform the glob search with optional path parameter
                let result = if let Some(path) = &params.path {
                    let path_buf = PathBuf::from(path);
                    SearchTools::glob_search_in_dir(&path_buf, &params.pattern)
                } else {
                    SearchTools::glob_search(&params.pattern)
                };

                match result {
                    Ok(results) => {
                        // Format the output
                        let mut output = format!(
                            "Found {} files matching pattern '{}':\n\n",
                            results.len(),
                            params.pattern
                        );
                        for (i, path) in results.iter().enumerate() {
                            output.push_str(&format!("{}. {}\n", i + 1, path.display()));
                        }

                        // Send success notification with count, pattern, and optional path
                        let metadata = if let Some(path) = &params.path {
                            serde_json::json!({
                                "pattern": params.pattern,
                                "path": path,
                                "count": results.len(),
                                "description": format!("Found {} files", results.len()),
                            })
                        } else {
                            serde_json::json!({
                                "pattern": params.pattern,
                                "count": results.len(),
                                "description": format!("Found {} files", results.len()),
                            })
                        };
                        send_tool_notification(
                            "Search",
                            "success",
                            &format!("Found {} files", results.len()),
                            metadata,
                            &tool_id,
                            start_time,
                        )
                        .ok();

                        Ok(output)
                    }
                    Err(e) => {
                        // Send error notification with pattern and optional path included
                        let metadata = if let Some(path) = &params.path {
                            serde_json::json!({
                                "pattern": params.pattern,
                                "path": path,
                                "description": format!("Error searching for pattern: {}", e),
                                // Explicitly include pattern field to ensure UI can access it
                                "pattern": params.pattern,
                            })
                        } else {
                            serde_json::json!({
                                "pattern": params.pattern,
                                "description": format!("Error searching for pattern: {}", e),
                                // Explicitly include pattern field to ensure UI can access it
                                "pattern": params.pattern,
                            })
                        };
                        send_tool_notification(
                            "Search",
                            "error",
                            &format!("Error searching for pattern: {}", e),
                            metadata,
                            &tool_id,
                            start_time,
                        )
                        .ok();

                        Err(e)
                    }
                }
            }
            ToolCall::GrepTool(params) => {
                // Generate a unique ID for this execution
                let tool_id = format!(
                    "grep-direct-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );

                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();

                // Construct metadata based on available parameters
                let description = match (&params.path, &params.include) {
                    (Some(path), Some(include)) => format!(
                        "Search(pattern: \"{}\", path: \"{}\", include: \"{}\")",
                        params.pattern, path, include
                    ),
                    (Some(path), None) => format!(
                        "Search(pattern: \"{}\", path: \"{}\")",
                        params.pattern, path
                    ),
                    (None, Some(include)) => format!(
                        "Search(pattern: \"{}\", include: \"{}\")",
                        params.pattern, include
                    ),
                    (None, None) => format!("Search(pattern: \"{}\")", params.pattern),
                };

                // Send start notification
                let metadata = serde_json::json!({
                    "pattern": params.pattern,
                    "include": params.include,
                    "path": params.path,
                    "description": description,
                });
                // Create a user-friendly message
                let message = match (&params.path, &params.include) {
                    (Some(path), Some(include)) => format!(
                        "Searching in {} for content: \"{}\" in files matching \"{}\"",
                        path, params.pattern, include
                    ),
                    (Some(path), None) => {
                        format!("Searching in {} for content: \"{}\"", path, params.pattern)
                    }
                    (None, Some(include)) => format!(
                        "Searching for content: \"{}\" in files matching \"{}\"",
                        params.pattern, include
                    ),
                    (None, None) => format!("Searching for content: \"{}\"", params.pattern),
                };

                send_tool_notification(
                    "Search", "running", &message, metadata, &tool_id, start_time,
                )
                .ok();

                // Add a brief delay to ensure the running state is visible
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Execute the grep search
                let search_dir = params.path.as_ref().map(Path::new);
                let result = SearchTools::grep_search(
                    &params.pattern,
                    params.include.as_deref(),
                    search_dir,
                );

                match result {
                    Ok(results) => {
                        // Format the output
                        let mut output = format!(
                            "Found {} matches for pattern '{}':\n\n",
                            results.len(),
                            params.pattern
                        );
                        for (path, line_num, line) in &results {
                            output.push_str(&format!("{}:{}:{}\n", path.display(), line_num, line));
                        }

                        // Send success notification
                        let metadata = serde_json::json!({
                            "pattern": params.pattern,
                            "include": params.include,
                            "path": params.path,
                            "count": results.len(),
                            "description": format!("Found {} files", results.len()),
                        });
                        send_tool_notification(
                            "Search",
                            "success",
                            &format!("Found {} files", results.len()),
                            metadata,
                            &tool_id,
                            start_time,
                        )
                        .ok();

                        Ok(output)
                    }
                    Err(e) => {
                        // Send error notification
                        let metadata = serde_json::json!({
                            "pattern": params.pattern,
                            "include": params.include,
                            "path": params.path,
                            "description": format!("Error searching content: {}", e),
                        });
                        send_tool_notification(
                            "Search",
                            "error",
                            &format!("Error searching content: {}", e),
                            metadata,
                            &tool_id,
                            start_time,
                        )
                        .ok();

                        Err(e)
                    }
                }
            }
            ToolCall::LSTool(params) => {
                // Generate a unique ID for this execution
                let tool_id = format!(
                    "listdir-direct-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );

                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();

                // Send start notification
                let metadata = serde_json::json!({
                    "path": params.path,
                    "file_path": params.path,
                    "description": format!("Listing directory: {}", params.path),
                });
                send_tool_notification(
                    "List",
                    "running",
                    &format!("Listing directory: {}", params.path),
                    metadata,
                    &tool_id,
                    start_time,
                )
                .ok();

                // Add a brief delay to ensure the running state is visible
                std::thread::sleep(std::time::Duration::from_millis(500));

                // List the directory
                let path = PathBuf::from(&params.path);
                let result = FileOps::list_directory(&path);

                match result {
                    Ok(entries) => {
                        // Build the output format
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

                        // Send success notification
                        let metadata = serde_json::json!({
                            "path": params.path,
                            "file_path": params.path,
                            "count": entries.len(),
                            "description": format!("Listed {} paths", entries.len()),
                        });
                        send_tool_notification(
                            "List",
                            "success",
                            &format!("Listed {} paths", entries.len()),
                            metadata,
                            &tool_id,
                            start_time,
                        )
                        .ok();

                        Ok(output)
                    }
                    Err(e) => {
                        // Send error notification
                        let metadata = serde_json::json!({
                            "path": params.path,
                            "file_path": params.path,
                            "description": format!("Error listing directory: {}", e),
                        });
                        send_tool_notification(
                            "List",
                            "error",
                            &format!("Error listing directory: {}", e),
                            metadata,
                            &tool_id,
                            start_time,
                        )
                        .ok();

                        Err(e)
                    }
                }
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
            ToolCall::DocumentSymbol(params) => {
                // Initialize LSP server manager
                let lsp_manager = LspServerManager::new();

                // Get document symbols
                let symbols =
                    lsp_manager.document_symbol(&params.file_path, &params.server_type)?;

                // Format the result
                let mut output = format!("Document symbols for '{}':\n\n", params.file_path);

                fn format_symbols(
                    symbols: &[crate::tools::lsp::DocumentSymbol],
                    depth: usize,
                    output: &mut String,
                ) {
                    for symbol in symbols {
                        // Add indentation based on depth
                        let indent = "  ".repeat(depth);

                        // Add symbol information
                        output.push_str(&format!(
                            "{}{} - {}\n",
                            indent,
                            symbol
                                .kind
                                .to_string()
                                .unwrap_or_else(|| format!("{:?}", symbol.kind)),
                            symbol.name
                        ));

                        // Add detail if available
                        if let Some(ref detail) = symbol.detail {
                            output.push_str(&format!("{}  Detail: {}\n", indent, detail));
                        }

                        // Recursively add children
                        if let Some(ref children) = symbol.children {
                            format_symbols(children, depth + 1, output);
                        }
                    }
                }

                format_symbols(&symbols, 0, &mut output);
                Ok(output)
            }
            ToolCall::SemanticTokens(params) => {
                // Initialize LSP server manager
                let lsp_manager = LspServerManager::new();

                // Get semantic tokens
                let tokens = lsp_manager.semantic_tokens(&params.file_path, &params.server_type)?;

                // Format the result
                let mut output = format!("Semantic tokens for '{}':\n\n", params.file_path);

                // Add tokens data
                output.push_str(&format!(
                    "Received {} token data points\n",
                    tokens.data.len() / 5
                ));

                // LSP semantic tokens are encoded as 5-tuples
                for chunk in tokens.data.chunks(5) {
                    if chunk.len() == 5 {
                        output.push_str(&format!(
                            "Token: delta_line={}, delta_start={}, length={}, token_type={}, token_modifiers={}\n",
                            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4]
                        ));
                    }
                }

                Ok(output)
            }
            ToolCall::CodeLens(params) => {
                // Initialize LSP server manager
                let lsp_manager = LspServerManager::new();

                // Get code lenses
                let lenses = lsp_manager.code_lens(&params.file_path, &params.server_type)?;

                // Format the result
                let mut output = format!("Code lenses for '{}':\n\n", params.file_path);

                for (i, lens) in lenses.iter().enumerate() {
                    output.push_str(&format!(
                        "{}. Range: {}:{} to {}:{}\n",
                        i + 1,
                        lens.range.start.line,
                        lens.range.start.character,
                        lens.range.end.line,
                        lens.range.end.character
                    ));

                    if let Some(ref command) = lens.command {
                        output.push_str(&format!("   Command: {}\n", command.title));
                        output.push_str(&format!("   Action: {}\n", command.command));
                    }

                    output.push('\n');
                }

                Ok(output)
            }
            ToolCall::Definition(params) => {
                // Initialize LSP server manager
                let lsp_manager = LspServerManager::new();

                // Get definition
                let locations = lsp_manager.definition(
                    &params.file_path,
                    &params.position,
                    &params.server_type,
                )?;

                // Format the result
                let mut output = format!(
                    "Definitions for position {}:{} in '{}':\n\n",
                    params.position.line, params.position.character, params.file_path
                );

                for (i, location) in locations.iter().enumerate() {
                    let uri = location.uri.replace("file://", "");

                    output.push_str(&format!("{}. File: {}\n", i + 1, uri));
                    output.push_str(&format!(
                        "   Range: {}:{} to {}:{}\n\n",
                        location.range.start.line,
                        location.range.start.character,
                        location.range.end.line,
                        location.range.end.character
                    ));
                }

                Ok(output)
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
            "description": "Fast file pattern matching tool using glob patterns like '**/*.rs', supports * (matches characters), ** (recursive directories), {} (alternatives)",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files against"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in (defaults to current directory)"
                    }
                },
                "required": ["pattern"]
            }
        }),
        serde_json::json!({
            "name": "GrepTool",
            "description": "Fast content search tool using regular expressions to find patterns in file contents",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regular expression pattern to search for in file contents"
                    },
                    "include": {
                        "type": "string",
                        "description": "File pattern to filter results (e.g. \"*.rs\", \"*.{ts,tsx}\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in (defaults to current directory)"
                    }
                },
                "required": ["pattern"]
            }
        }),
        serde_json::json!({
            "name": "LSTool",
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
        serde_json::json!({
            "name": "DocumentSymbol",
            "description": "Extracts document symbols from a file using LSP",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to analyze"
                    },
                    "server_type": {
                        "type": "string",
                        "enum": ["Python", "Rust"],
                        "description": "The type of LSP server to use"
                    }
                },
                "required": ["file_path", "server_type"]
            }
        }),
        serde_json::json!({
            "name": "SemanticTokens",
            "description": "Extracts semantic tokens from a file using LSP",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to analyze"
                    },
                    "server_type": {
                        "type": "string",
                        "enum": ["Python", "Rust"],
                        "description": "The type of LSP server to use"
                    }
                },
                "required": ["file_path", "server_type"]
            }
        }),
        serde_json::json!({
            "name": "CodeLens",
            "description": "Extracts code lenses from a file using LSP",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to analyze"
                    },
                    "server_type": {
                        "type": "string",
                        "enum": ["Python", "Rust"],
                        "description": "The type of LSP server to use"
                    }
                },
                "required": ["file_path", "server_type"]
            }
        }),
        serde_json::json!({
            "name": "Definition",
            "description": "Finds the definition of a symbol at a specific position in a file using LSP",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to analyze"
                    },
                    "position": {
                        "type": "object",
                        "properties": {
                            "line": {
                                "type": "integer",
                                "description": "The line number (0-based)"
                            },
                            "character": {
                                "type": "integer",
                                "description": "The character position (0-based)"
                            }
                        },
                        "required": ["line", "character"],
                        "description": "The position of the symbol in the file"
                    },
                    "server_type": {
                        "type": "string",
                        "enum": ["Python", "Rust"],
                        "description": "The type of LSP server to use"
                    }
                },
                "required": ["file_path", "position", "server_type"]
            }
        }),
    ]
}
