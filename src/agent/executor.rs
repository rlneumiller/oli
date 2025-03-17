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

    #[allow(dead_code)]
    pub fn add_assistant_message(&mut self, content: String) {
        self.conversation.push(Message::assistant(content));
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

        // Update progress if sender is configured
        if let Some(sender) = &self.progress_sender {
            let _ = sender
                .send("[wait] Sending request to AI assistant...".to_string())
                .await;
        }

        // Execute the first completion with tools
        let (content, tool_calls) = self
            .api_client
            .complete_with_tools(self.conversation.clone(), options.clone(), None)
            .await?;

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

            // Update progress
            if let Some(sender) = &self.progress_sender {
                let _ = sender
                    .send(format!("Executing {} tool calls...", calls.len()))
                    .await;
            }

            // Execute each tool call and collect results
            for (i, call) in calls.iter().enumerate() {
                if let Some(sender) = &self.progress_sender {
                    let _ = sender
                        .send(format!("[tool] Running tool {}: {}...", i + 1, call.name))
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
                        // Use the ID from the tool call if available
                        tool_results.push(ToolResult {
                            tool_call_id: call.id.clone().unwrap_or_else(|| i.to_string()),
                            output: format!("ERROR PARSING TOOL CALL: {}. Please check the format of your arguments and try again.", e),
                        });
                        continue;
                    }
                };

                // Show tool selection message
                if let Some(sender) = &self.progress_sender {
                    let _ = sender
                        .send(format!(
                            "[tool] Using tool {} with arguments: {:?}",
                            call.name, call.arguments
                        ))
                        .await;
                }

                // Execute the tool
                let result = match tool_call.execute() {
                    Ok(output) => {
                        // Show success message with output preview
                        if let Some(sender) = &self.progress_sender {
                            let preview = if output.len() > 200 {
                                format!("{}... (truncated)", &output[..200])
                            } else {
                                output.clone()
                            };
                            let _ = sender
                                .send(format!("[success] Tool result: {}", preview))
                                .await;
                        }
                        output
                    }
                    Err(e) => {
                        let error_msg = format!("Tool execution failed: {}", e);
                        if let Some(sender) = &self.progress_sender {
                            let _ = sender.send(format!("[error] {}", error_msg)).await;
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

            // Update progress
            if let Some(sender) = &self.progress_sender {
                let _ = sender
                    .send(format!(
                        "[wait] Processing {} tool results...",
                        tool_results.len()
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
