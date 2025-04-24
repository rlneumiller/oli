use crate::agent::tools::{get_tool_definitions, ToolCall};
use crate::apis::api_client::{
    CompletionOptions, DynApiClient, Message, ToolDefinition, ToolResult,
};
use anyhow::{Context, Result};
use serde_json::{self, Value};
use tokio::sync::mpsc;

// We'll implement token usage tracking directly in the app without
// a separate structure for now

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

    pub fn set_conversation_history(&mut self, history: Vec<Message>) {
        self.conversation = history;
    }

    pub fn get_conversation_history(&self) -> Vec<Message> {
        self.conversation.clone()
    }

    /// Analyze conversation to determine if code parsing might be needed
    /// Uses the LLM directly to make this determination rather than keyword matching
    async fn might_need_codebase_parsing(&self) -> Result<bool> {
        // Get the latest user message
        if let Some(last_user_msg) = self
            .conversation
            .iter()
            .rev()
            .find(|msg| msg.role == "user")
        {
            let content = &last_user_msg.content;

            // Create a system message explaining the task
            let system_message = Message::system(
                "You are an assistant that analyzes user queries to determine if they require \
                code structure understanding. Respond with only 'yes' or 'no'. Answer 'yes' if \
                the query involves understanding, modifying, or implementing code structures like \
                functions, classes, modules, etc. Answer 'no' for general information queries, tool \
                usage questions, or non-code tasks.".to_string()
            );

            // Create a user message with the query
            let query_message = Message::user(format!(
                "Based solely on this query, would understanding the code structure be helpful? \
                Query: '{}'",
                content
            ));

            // Create a mini-conversation for this specific task
            let mini_conversation = vec![system_message, query_message];

            // Create LLM options with minimal settings
            let options = CompletionOptions {
                temperature: Some(0.1), // Low temperature for deterministic response
                top_p: Some(0.95),
                max_tokens: Some(10), // Very small response needed
                tools: None,          // No tools needed
                require_tool_use: false,
                json_schema: None,
            };

            // Call the API to get the determination - using a separate client call
            // that doesn't affect our main conversation history
            let (response, _) = self
                .api_client
                .complete_with_tools(mini_conversation, options, None)
                .await?;

            // Check the response - looking for a "yes" answer
            let response_lower = response.to_lowercase();
            Ok(response_lower.contains("yes") || response_lower.contains("true"))
        } else {
            // No user message found
            Ok(false)
        }
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

        // Check if the query might need codebase parsing using the LLM
        // This initial check helps determine if we should suggest code parsing to the model

        let needs_parsing = self.might_need_codebase_parsing().await?;

        // If this query potentially needs code parsing, suggest it to the model
        // by adding a system hint for better context understanding
        if needs_parsing {
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

            // We don't need this summary since each tool call will have its own message
            // This improves the async nature of the tool execution

            // Execute each tool call and collect results
            for (i, call) in calls.iter().enumerate() {
                // Send basic tool execution message to UI
                if let Some(sender) = &self.progress_sender {
                    let _ = sender
                        .send(format!("⏺ [{}] Executing {}...", call.name, call.name))
                        .await;
                }

                // Parse the tool call into our enum
                let tool_call: ToolCall = match parse_tool_call(&call.name, &call.arguments) {
                    Ok(tc) => tc,
                    Err(e) => {
                        let error_msg = format!("Failed to parse tool call: {}", e);
                        if let Some(sender) = &self.progress_sender {
                            let _ = sender.send(format!("[error] {}", error_msg)).await;
                        }

                        // Instead of returning error, provide helpful error message to the model
                        // Use the ID from the tool call if available (ensuring it's valid for Anthropic API)
                        tool_results.push(ToolResult {
                            tool_call_id: call.id.clone().unwrap_or_else(|| format!("tool_{}", i)),
                            output: format!("ERROR PARSING TOOL CALL: {}. Please check the format of your arguments and try again.", e),
                        });
                        continue;
                    }
                };

                // No need for additional tool selection message since we already show it above
                // This prevents redundant messages about tool usage

                // For Edit and Replace tools, show diff preview before executing
                let needs_diff_preview = matches!(call.name.as_str(), "Edit" | "Replace");

                // Execute the tool
                let result = match if needs_diff_preview {
                    // For file modifications, preview the diff first, then request permission and execute
                    match &tool_call {
                        ToolCall::Edit(params) => {
                            use crate::tools::fs::file_ops::FileOps;
                            use std::path::PathBuf;

                            // Generate the diff without making changes yet
                            let path = PathBuf::from(&params.file_path);
                            match FileOps::generate_edit_diff(
                                &path,
                                &params.old_string,
                                &params.new_string,
                            ) {
                                Ok((_, diff)) => {
                                    // Send the diff as a progress message for the permission system to pick up
                                    if let Some(sender) = &self.progress_sender {
                                        let _ = sender.send(diff.clone()).await;
                                    }
                                    // Now execute the actual tool to make the changes
                                    tool_call.execute()
                                }
                                Err(e) => Err(e),
                            }
                        }
                        ToolCall::Replace(params) => {
                            use crate::tools::fs::file_ops::FileOps;
                            use std::path::PathBuf;

                            // Generate the diff without making changes yet
                            let path = PathBuf::from(&params.file_path);
                            match FileOps::generate_write_diff(&path, &params.content) {
                                Ok((diff, _)) => {
                                    // Send the diff as a progress message for the permission system to pick up
                                    if let Some(sender) = &self.progress_sender {
                                        let _ = sender.send(diff.clone()).await;
                                    }
                                    // Now execute the actual tool to make the changes
                                    tool_call.execute()
                                }
                                Err(e) => Err(e),
                            }
                        }
                        _ => tool_call.execute(), // Shouldn't happen, but fallback
                    }
                } else {
                    // For non-file operations, execute normally
                    tool_call.execute()
                } {
                    Ok(output) => {
                        // Send a special marker for tool counting that's easy to detect
                        if let Some(sender) = &self.progress_sender {
                            let _ = sender.send("[TOOL_EXECUTED]".to_string()).await;
                        }

                        // No need for additional notifications here - tools.rs already sends detailed notifications
                        // through the RPC system with proper tool status updates

                        output
                    }
                    Err(e) => {
                        // The tools.rs implementation already sends detailed error notifications via RPC
                        // Return error message as tool result without additional notifications
                        format!("ERROR EXECUTING TOOL: {}", e)
                    }
                };

                // Create a valid tool result ID (for Anthropic API: only alphanumeric, underscore, hyphen)
                let tool_call_id = call.id.clone().unwrap_or_else(|| format!("tool_{}", i));

                // Add tool result with proper ID for API compatibility
                tool_results.push(ToolResult {
                    tool_call_id: tool_call_id.clone(),
                    output: result.clone(),
                });

                // Also add a user message with the tool result to maintain history properly
                self.conversation.push(Message {
                    role: "user".to_string(),
                    content: format!("Tool result for call {}: {}", tool_call_id, result),
                });
            }

            // Don't show "processing" message to reduce UI noise

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
        "FileReadTool" => {
            let params = serde_json::from_value(args.clone())
                .context("Failed to parse FileReadTool parameters")?;
            Ok(ToolCall::FileReadTool(params))
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
        "LSTool" => {
            let params = serde_json::from_value(args.clone())
                .context("Failed to parse LSTool parameters")?;
            Ok(ToolCall::LSTool(params))
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

// Note: For future improvements, we could extract actual token usage from the API responses
// rather than estimating them based on string length. This would involve parsing the
// response JSON to extract token counts from both Anthropic and OpenAI formats.
