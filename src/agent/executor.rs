use crate::agent::tools::{get_tool_definitions, ToolCall};
use crate::apis::api_client::{
    CompletionOptions, DynApiClient, Message, ToolDefinition, ToolResult,
};
use anyhow::{Context, Result};
use serde_json::{self, Value};
use tokio::sync::mpsc;

pub struct AgentExecutor {
    api_client: DynApiClient,
    conversation: Vec<Message>,
    tool_definitions: Vec<ToolDefinition>,
    progress_sender: Option<mpsc::Sender<String>>,
}

impl AgentExecutor {
    pub fn new(api_client: DynApiClient) -> Self {
        let tool_defs = get_tool_definitions()
            .into_iter()
            .map(|def| ToolDefinition {
                name: def["name"].as_str().unwrap_or("").to_string(),
                description: def["description"].as_str().unwrap_or("").to_string(),
                parameters: def["parameters"].clone(),
            })
            .collect();

        Self {
            api_client,
            conversation: Vec::new(),
            tool_definitions: tool_defs,
            progress_sender: None,
        }
    }

    /// Analyze conversation to determine if code parsing might be needed
    fn might_need_codebase_parsing(&self) -> bool {
        // Get the latest user message
        if let Some(last_user_msg) = self
            .conversation
            .iter()
            .rev()
            .find(|msg| msg.role == "user")
        {
            let content = &last_user_msg.content;

            // Keywords that suggest codebase understanding might be needed
            let code_related_keywords = [
                "code",
                "implement",
                "function",
                "class",
                "method",
                "refactor",
                "improve",
                "optimize",
                "bug",
                "fix",
                "test",
                "create",
                "add",
                "modify",
                "update",
                "change",
                "remove",
                "file",
                "module",
                "package",
                "import",
                "dependency",
                "struct",
                "enum",
                "trait",
                "impl",
                "interface",
                "build",
                "compile",
                "run",
                "execute",
                "install",
                "architecture",
                "design",
                "pattern",
            ];

            // Check if multiple code-related keywords are present
            let keyword_count = code_related_keywords
                .iter()
                .filter(|&&kw| content.to_lowercase().contains(kw))
                .count();

            // If the query contains multiple code-related keywords, suggest parsing
            return keyword_count >= 2;
        }

        false
    }

    pub fn with_progress_sender(mut self, sender: mpsc::Sender<String>) -> Self {
        self.progress_sender = Some(sender);
        self
    }

    pub fn add_system_message(&mut self, content: String) {
        self.conversation.push(Message::system(content));
    }

    pub fn add_user_message(&mut self, content: String) {
        self.conversation.push(Message::user(content));
    }

    pub async fn execute(&mut self) -> Result<String> {
        // Create options with tools enabled and optimized parameters for Claude 3.7
        let options = CompletionOptions {
            temperature: Some(0.5), // Lower temperature for more precise outputs
            top_p: Some(0.95),      // Slightly higher top_p for better quality
            max_tokens: Some(4096), // Generous token limit
            tools: Some(self.tool_definitions.clone()),
            require_tool_use: false, // Let the model decide when to use tools
            json_schema: None,       // No structured format for initial response
        };

        // Check if the query might need codebase parsing
        // This initial check helps determine if we should suggest code parsing to the model
        let needs_parsing = self.might_need_codebase_parsing();

        // Update progress if sender is configured with real-time status
        if let Some(sender) = &self.progress_sender {
            let _ = sender
                .send("⏺ Sending request to AI assistant...".to_string())
                .await;
        }

        // If this query potentially needs code parsing, suggest it to the model
        // by adding a system hint for better context understanding
        if needs_parsing {
            if let Some(sender) = &self.progress_sender {
                let _ = sender
                    .send("⏺ Analyzing if codebase parsing is needed...".to_string())
                    .await;
            }

            // Add a temporary system message suggesting code parsing
            self.conversation.push(Message::system(
                "The user's query appears to be related to code. Consider using the ParseCode tool \
                to understand the codebase structure before responding, if you need to understand \
                the code to provide a solution. The ParseCode tool will generate an AST \
                (Abstract Syntax Tree) that helps you understand the code structure.".to_string()
            ));
        }

        // Execute the first completion with tools
        let (content, tool_calls) = self
            .api_client
            .complete_with_tools(self.conversation.clone(), options.clone(), None)
            .await?;

        // Remove the temporary system message if it was added
        if needs_parsing {
            if let Some(last) = self.conversation.last() {
                if last.role == "system" && last.content.contains("ParseCode tool") {
                    self.conversation.pop();
                }
            }
        }

        // If there are no tool calls, add the content to conversation and return
        if tool_calls.is_none() {
            self.conversation.push(Message::assistant(content.clone()));
            return Ok(content);
        }

        // Add the assistant's message with tool calls to the conversation - important for OpenAI API
        // We need to preserve all the context including tool calls for proper API behavior

        // For OpenAI compatibility, store the tool calls in the message content as structured JSON
        // This allows for proper serialization/deserialization of tool calls in the message history
        if let Some(calls) = &tool_calls {
            // Create a JSON object with both content and tool calls
            let message_with_tools = serde_json::json!({
                "content": content,
                "tool_calls": calls.iter().map(|call| {
                    serde_json::json!({
                        "id": call.id.clone().unwrap_or_default(),
                        "name": call.name,
                        "arguments": call.arguments
                    })
                }).collect::<Vec<_>>()
            });

            // Store as JSON string in the message
            self.conversation.push(Message::assistant(
                serde_json::to_string(&message_with_tools).unwrap_or_else(|_| content.clone()),
            ));
        } else {
            // No tool calls, just store the content directly
            self.conversation.push(Message::assistant(content.clone()));
        }

        // Process tool calls in a loop until no more tools are called
        let mut current_content = content;
        let mut current_tool_calls = tool_calls;
        let mut tool_results = Vec::new();
        let mut loop_count = 0;
        const MAX_LOOPS: usize = 10; // Safety limit for tool call loops

        while let Some(ref calls) = current_tool_calls {
            // Safety check to prevent infinite loops
            loop_count += 1;
            if loop_count > MAX_LOOPS {
                if let Some(sender) = &self.progress_sender {
                    let _ = sender
                        .send("Reached maximum number of tool call loops. Stopping.".to_string())
                        .await;
                }
                break;
            }

            // Update progress with real-time status
            if let Some(sender) = &self.progress_sender {
                let _ = sender
                    .send(format!(
                        "⏺ Executing {} tool call{}...",
                        calls.len(),
                        if calls.len() == 1 { "" } else { "s" }
                    ))
                    .await;
            }

            // Execute each tool call and collect results
            for (i, call) in calls.iter().enumerate() {
                // Format tool call details for better UI display before execution
                let formatted_tool_details = match call.name.as_str() {
                    "View" => {
                        if let (Some(path), Some(offset), Some(limit)) = (
                            call.arguments.get("file_path").and_then(|v| v.as_str()),
                            call.arguments.get("offset").and_then(|v| v.as_u64()),
                            call.arguments.get("limit").and_then(|v| v.as_u64()),
                        ) {
                            format!(
                                "View(file_path: \"{}\", offset: {}, limit: {})…",
                                path, offset, limit
                            )
                        } else if let Some(path) =
                            call.arguments.get("file_path").and_then(|v| v.as_str())
                        {
                            format!("View(file_path: \"{}\")…", path)
                        } else {
                            format!("View({:?})…", call.arguments)
                        }
                    }
                    "GlobTool" => {
                        if let (Some(pattern), Some(path)) = (
                            call.arguments.get("pattern").and_then(|v| v.as_str()),
                            call.arguments.get("path").and_then(|v| v.as_str()),
                        ) {
                            format!("GlobTool(pattern: \"{}\", path: \"{}\")…", pattern, path)
                        } else if let Some(pattern) =
                            call.arguments.get("pattern").and_then(|v| v.as_str())
                        {
                            format!("GlobTool(pattern: \"{}\")…", pattern)
                        } else {
                            format!("GlobTool({:?})…", call.arguments)
                        }
                    }
                    "GrepTool" => {
                        if let Some(pattern) =
                            call.arguments.get("pattern").and_then(|v| v.as_str())
                        {
                            format!("GrepTool(pattern: \"{}\")…", pattern)
                        } else {
                            format!("GrepTool({:?})…", call.arguments)
                        }
                    }
                    "LS" => {
                        if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                            format!("LS(path: \"{}\")…", path)
                        } else {
                            format!("LS({:?})…", call.arguments)
                        }
                    }
                    "Edit" | "Replace" => {
                        if let Some(path) = call.arguments.get("file_path").and_then(|v| v.as_str())
                        {
                            format!("{} file: \"{}\"…", call.name, path)
                        } else {
                            format!("{} {:?}…", call.name, call.arguments)
                        }
                    }
                    "Bash" => {
                        if let Some(cmd) = call.arguments.get("command").and_then(|v| v.as_str()) {
                            if cmd.len() > 40 {
                                format!("Bash(command: \"{}...\")…", &cmd[..40])
                            } else {
                                format!("Bash(command: \"{}\")…", cmd)
                            }
                        } else {
                            format!("Bash({:?})…", call.arguments)
                        }
                    }
                    _ => format!("{} {:?}…", call.name, call.arguments),
                };

                // Send the formatted tool details to UI before execution
                if let Some(sender) = &self.progress_sender {
                    let _ = sender
                        .send(format!("\x1b[32m⏺\x1b[0m {}", formatted_tool_details))
                        .await;
                }

                // Use the previously formatted tool details for showing execution

                // Parse the tool call into our enum
                let tool_call: ToolCall = match parse_tool_call(&call.name, &call.arguments) {
                    Ok(tc) => tc,
                    Err(e) => {
                        let error_msg = format!("Failed to parse tool call: {}", e);
                        if let Some(sender) = &self.progress_sender {
                            let _ = sender.send(format!("[error] {}", error_msg)).await;
                        }

                        // Instead of returning error, provide helpful error message to the model
                        // Use the ID from the tool call if available
                        tool_results.push(ToolResult {
                            tool_call_id: call.id.clone().unwrap_or_else(|| i.to_string()),
                            output: format!("ERROR PARSING TOOL CALL: {}. Please check the format of your arguments and try again.", e),
                        });
                        continue;
                    }
                };

                // No need for additional tool selection message since we already show it above
                // This prevents redundant messages about tool usage

                // Execute the tool
                let result = match tool_call.execute() {
                    Ok(output) => {
                        // Format successful tool result with detailed output
                        if let Some(sender) = &self.progress_sender {
                            // Create a preview of the output
                            let preview = if output.len() > 200 {
                                format!(
                                    "{}... [+{} more chars]",
                                    &output[..200],
                                    output.len() - 200
                                )
                            } else {
                                output.clone()
                            };

                            // For file outputs, prepare a structured display for the UI
                            let formatted_result = match call.name.as_str() {
                                "View" => {
                                    if let Some(path) =
                                        call.arguments.get("file_path").and_then(|v| v.as_str())
                                    {
                                        // Display path and first few content lines
                                        let output_lines: Vec<&str> = output.lines().collect();
                                        let header =
                                            format!("View(file_path: \"{}\") → Result:", path);

                                        // Format first few lines with line numbers
                                        if output_lines.len() <= 2 {
                                            format!("{}\n  ⎿ {}", header, output)
                                        } else {
                                            let mut formatted = header.to_string();
                                            for (i, line) in output_lines.iter().take(3).enumerate()
                                            {
                                                formatted.push_str(&format!("\n  ⎿ {}", line));
                                                if i == 2 && output_lines.len() > 3 {
                                                    formatted.push_str(&format!(
                                                        "\n  ... [{} more lines]",
                                                        output_lines.len() - 3
                                                    ));
                                                }
                                            }
                                            formatted
                                        }
                                    } else {
                                        format!("Tool result: {}", preview)
                                    }
                                }
                                "GlobTool" | "GrepTool" => {
                                    let pattern = call
                                        .arguments
                                        .get("pattern")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown");
                                    let name = if call.name.as_str() == "GlobTool" {
                                        "Glob"
                                    } else {
                                        "Grep"
                                    };
                                    format!("{}(pattern: \"{}\") → Found {} files\n  ⎿ First results shown in output",
                                        name, pattern,
                                        output.lines().filter(|l| l.contains(".")).count())
                                }
                                "LS" => {
                                    if let Some(path) =
                                        call.arguments.get("path").and_then(|v| v.as_str())
                                    {
                                        let file_count =
                                            output.lines().filter(|l| l.contains("FILE")).count();
                                        let dir_count =
                                            output.lines().filter(|l| l.contains("DIR")).count();
                                        format!("LS(path: \"{}\") → Listed {} items ({} files, {} dirs)", 
                                            path, file_count + dir_count, file_count, dir_count)
                                    } else {
                                        format!("Tool result: {}", preview)
                                    }
                                }
                                "Edit" | "Replace" => {
                                    if let Some(path) =
                                        call.arguments.get("file_path").and_then(|v| v.as_str())
                                    {
                                        format!("{} file: \"{}\" → {}", call.name, path, output)
                                    } else {
                                        format!("Tool result: {}", preview)
                                    }
                                }
                                "Bash" => {
                                    if let Some(cmd) =
                                        call.arguments.get("command").and_then(|v| v.as_str())
                                    {
                                        let cmd_preview = if cmd.len() > 30 {
                                            format!("{}...", &cmd[..30])
                                        } else {
                                            cmd.to_string()
                                        };

                                        // Format command output with line counts
                                        let output_lines = output.lines().count();
                                        if output_lines > 5 {
                                            let mut formatted = format!(
                                                "Bash(command: \"{}\") → {} lines of output:",
                                                cmd_preview, output_lines
                                            );
                                            for line in output.lines().take(3) {
                                                formatted.push_str(&format!("\n  ⎿ {}", line));
                                            }
                                            formatted.push_str(&format!(
                                                "\n  ... [{} more lines]",
                                                output_lines - 3
                                            ));
                                            formatted
                                        } else if output_lines > 0 {
                                            let mut formatted = format!(
                                                "Bash(command: \"{}\") → Output:",
                                                cmd_preview
                                            );
                                            for line in output.lines() {
                                                formatted.push_str(&format!("\n  ⎿ {}", line));
                                            }
                                            formatted
                                        } else {
                                            format!(
                                                "Bash(command: \"{}\") → No output",
                                                cmd_preview
                                            )
                                        }
                                    } else {
                                        format!("Tool result: {}", preview)
                                    }
                                }
                                _ => format!("Tool result: {}", preview),
                            };

                            let _ = sender
                                .send(format!("\x1b[32m⏺\x1b[0m {}", formatted_result))
                                .await;

                            // Small delay to allow UI update
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        output
                    }
                    Err(e) => {
                        let error_msg = format!("Tool execution failed: {}", e);
                        if let Some(sender) = &self.progress_sender {
                            let _ = sender.send(format!("\x1b[31m⏺\x1b[0m {}", error_msg)).await;
                        }

                        // Return error message as tool result
                        format!("ERROR EXECUTING TOOL: {}", e)
                    }
                };

                // Add tool result with proper ID for API compatibility
                tool_results.push(ToolResult {
                    // Use the ID from the tool call if available
                    tool_call_id: call.id.clone().unwrap_or_else(|| i.to_string()),
                    output: result,
                });
            }

            // Update progress with real-time status
            if let Some(sender) = &self.progress_sender {
                let _ = sender
                    .send(format!(
                        "⏺ Processing {} tool result{} and generating response...",
                        tool_results.len(),
                        if tool_results.len() == 1 { "" } else { "s" }
                    ))
                    .await;
            }

            // For subsequent calls, add the tool results and use JSON schema to get more reliable output
            let next_options = if loop_count >= MAX_LOOPS - 1 {
                // On the last loop, request a final summary with no further tool calls
                CompletionOptions {
                    require_tool_use: false,
                    json_schema: Some(
                        r#"
                    {
                        "type": "object",
                        "properties": {
                            "finalSummary": {
                                "type": "string",
                                "description": "Final comprehensive summary of findings and results"
                            }
                        },
                        "required": ["finalSummary"]
                    }
                    "#
                        .to_string(),
                    ),
                    ..options.clone()
                }
            } else {
                // For intermediate calls, continue with normal options
                options.clone()
            };

            // Request another completion with the tool results
            let (next_content, next_tool_calls) = self
                .api_client
                .complete_with_tools(
                    self.conversation.clone(),
                    next_options,
                    Some(tool_results.clone()),
                )
                .await?;

            // Extract JSON content if present
            current_content = if next_content.trim().starts_with('{')
                && next_content.trim().ends_with('}')
            {
                // Try to parse as JSON to extract the finalSummary if available
                match serde_json::from_str::<serde_json::Value>(&next_content) {
                    Ok(json) => {
                        if let Some(summary) = json.get("finalSummary").and_then(|s| s.as_str()) {
                            summary.to_string()
                        } else {
                            next_content
                        }
                    }
                    Err(_) => next_content,
                }
            } else {
                next_content
            };

            current_tool_calls = next_tool_calls;

            // If no more tool calls, break the loop
            if current_tool_calls.is_none() {
                break;
            }

            // Clear previous tool results
            tool_results.clear();
        }

        // Add final response to conversation
        // Handle the case where there might still be tool calls
        if let Some(calls) = &current_tool_calls {
            // Create a JSON object with both content and tool calls
            let message_with_tools = serde_json::json!({
                "content": current_content,
                "tool_calls": calls.iter().map(|call| {
                    serde_json::json!({
                        "id": call.id.clone().unwrap_or_default(),
                        "name": call.name,
                        "arguments": call.arguments
                    })
                }).collect::<Vec<_>>()
            });

            // Store as JSON string in the message
            self.conversation.push(Message::assistant(
                serde_json::to_string(&message_with_tools)
                    .unwrap_or_else(|_| current_content.clone()),
            ));
        } else {
            // No tool calls, just store the content directly
            self.conversation
                .push(Message::assistant(current_content.clone()));
        }

        Ok(current_content)
    }
}

fn parse_tool_call(name: &str, args: &Value) -> Result<ToolCall> {
    match name {
        "View" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse View parameters")?;
            Ok(ToolCall::View(params))
        }
        "GlobTool" => {
            let params = serde_json::from_value(args.clone())
                .context("Failed to parse GlobTool parameters")?;
            Ok(ToolCall::GlobTool(params))
        }
        "GrepTool" => {
            let params = serde_json::from_value(args.clone())
                .context("Failed to parse GrepTool parameters")?;
            Ok(ToolCall::GrepTool(params))
        }
        "LS" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse LS parameters")?;
            Ok(ToolCall::LS(params))
        }
        "Edit" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Edit parameters")?;
            Ok(ToolCall::Edit(params))
        }
        "Replace" => {
            let params = serde_json::from_value(args.clone())
                .context("Failed to parse Replace parameters")?;
            Ok(ToolCall::Replace(params))
        }
        "Bash" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Bash parameters")?;
            Ok(ToolCall::Bash(params))
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}
