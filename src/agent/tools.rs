use crate::tools::{code::parser::CodeParser, fs::file_ops::FileOps, fs::search::SearchTools};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolType {
    View,
    GlobTool,
    GrepTool,
    LS,
    Edit,
    Replace,
    Bash,
    ParseCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewParams {
    pub file_path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
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
pub struct ParseCodeParams {
    pub root_dir: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "params")]
pub enum ToolCall {
    View(ViewParams),
    GlobTool(GlobToolParams),
    GrepTool(GrepToolParams),
    LS(LSParams),
    Edit(EditParams),
    Replace(ReplaceParams),
    Bash(BashParams),
    ParseCode(ParseCodeParams),
}

impl ToolCall {
    pub fn execute(&self) -> Result<String> {
        match self {
            ToolCall::View(params) => {
                let path = PathBuf::from(&params.file_path);
                if let (Some(offset), Some(limit)) = (params.offset, params.limit) {
                    FileOps::read_file_lines(&path, offset, Some(limit))
                } else {
                    FileOps::read_file_with_line_numbers(&path)
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
                FileOps::edit_file(&path, &params.old_string, &params.new_string)?;
                Ok(format!("Successfully edited file: {}", params.file_path))
            }
            ToolCall::Replace(params) => {
                let path = PathBuf::from(&params.file_path);
                FileOps::write_file(&path, &params.content)?;
                Ok(format!("Successfully replaced file: {}", params.file_path))
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
            ToolCall::ParseCode(params) => {
                let mut parser = CodeParser::new()?;
                let root_dir = PathBuf::from(&params.root_dir);

                // Generate AST data optimized for LLM consumption
                let ast_data = parser.generate_llm_friendly_ast(&root_dir, &params.query)?;

                // Return the AST data as JSON
                Ok(format!(
                    "Code structure parsed successfully. AST data:\n\n{}",
                    ast_data
                ))
            }
        }
    }
}

pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        serde_json::json!({
            "name": "View",
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
                        "description": "The line number to start reading from (optional)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The number of lines to read (optional)"
                    }
                },
                "required": ["file_path"]
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
        serde_json::json!({
            "name": "ParseCode",
            "description": "Parses codebase and generates AST for LLM understanding",
            "parameters": {
                "type": "object",
                "properties": {
                    "root_dir": {
                        "type": "string",
                        "description": "The root directory of the codebase to parse"
                    },
                    "query": {
                        "type": "string",
                        "description": "The user query to determine relevant files and context"
                    }
                },
                "required": ["root_dir", "query"]
            }
        }),
    ]
}
