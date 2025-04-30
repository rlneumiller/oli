use crate::agent::tools::{get_tool_definitions, ToolCall as AgentToolCall};
use crate::apis::api_client::{
    CompletionOptions, DynApiClient, Message, ToolCall as ApiToolCall, ToolDefinition, ToolResult,
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
        // Create options with tools enabled and optimized parameters
        let options = CompletionOptions {
            temperature: Some(0.25),
            top_p: Some(0.95),
            max_tokens: Some(4096),
            tools: Some(self.tool_definitions.clone()),
            require_tool_use: false,
            json_schema: None,
        };

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

        // Add the assistant's message with tool calls to the conversation
        add_assistant_message_to_conversation(&mut self.conversation, &content, &tool_calls);

        // Process tool calls in a loop until no more tools are called
        let mut current_content = content;
        let mut current_tool_calls = tool_calls;
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

            // Execute all tool calls
            let tool_results = self.execute_tool_calls(calls, loop_count).await;

            // Determine completion options based on loop count
            let next_options = get_next_completion_options(loop_count, MAX_LOOPS, &options);

            // Request another completion with the tool results
            let (next_content, next_tool_calls) = self
                .api_client
                .complete_with_tools(self.conversation.clone(), next_options, Some(tool_results))
                .await?;

            // Extract JSON content if present (for finalSummary if available)
            current_content = extract_content_from_response(&next_content);
            current_tool_calls = next_tool_calls;

            // If no more tool calls, break the loop
            if current_tool_calls.is_none() {
                break;
            }
        }

        // Add final response to conversation
        add_assistant_message_to_conversation(
            &mut self.conversation,
            &current_content,
            &current_tool_calls,
        );

        Ok(current_content)
    }

    async fn execute_tool_calls(
        &mut self,
        calls: &[ApiToolCall],
        _loop_count: usize,
    ) -> Vec<ToolResult> {
        let mut results = Vec::with_capacity(calls.len());

        for (i, call) in calls.iter().enumerate() {
            // Send tool execution progress message
            if let Some(sender) = &self.progress_sender {
                let _ = sender
                    .send(format!("⏺ [{}] Executing {}...", call.name, call.name))
                    .await;
            }

            // Parse the tool call into our enum
            let tool_call: AgentToolCall = match parse_tool_call(&call.name, &call.arguments) {
                Ok(tc) => tc,
                Err(e) => {
                    send_error_message(
                        &self.progress_sender,
                        &format!("Failed to parse tool call: {}", e),
                    )
                    .await;

                    // Add error result and continue to next tool call
                    let tool_call_id = call.id.clone().unwrap_or_else(|| format!("tool_{}", i));
                    let error_message = format!("ERROR PARSING TOOL CALL: {}. Please check the format of your arguments and try again.", e);

                    self.add_tool_result_to_conversation(&tool_call_id, &error_message);
                    results.push(ToolResult {
                        tool_call_id,
                        output: error_message,
                    });

                    continue;
                }
            };

            // Execute the tool with preview for file modification tools
            let result = execute_tool_with_preview(&tool_call, call, &self.progress_sender).await;

            // Create a valid tool result ID
            let tool_call_id = call.id.clone().unwrap_or_else(|| format!("tool_{}", i));

            // Send tool execution completed message
            if let Some(sender) = &self.progress_sender {
                let _ = sender.send("[TOOL_EXECUTED]".to_string()).await;
            }

            // Add tool result to conversation and results collection
            self.add_tool_result_to_conversation(&tool_call_id, &result);
            results.push(ToolResult {
                tool_call_id,
                output: result,
            });
        }

        results
    }

    fn add_tool_result_to_conversation(&mut self, tool_call_id: &str, result: &str) {
        self.conversation.push(Message {
            role: "user".to_string(),
            content: format!("Tool result for call {}: {}", tool_call_id, result),
        });
    }
}

// Helper functions to improve readability

fn add_assistant_message_to_conversation(
    conversation: &mut Vec<Message>,
    content: &str,
    tool_calls: &Option<Vec<ApiToolCall>>,
) {
    if let Some(calls) = tool_calls {
        // Create a JSON object with both content and tool calls
        let message_with_tools = serde_json::json!({
            "content": content,
            "tool_calls": calls.iter().map(|call| {
                serde_json::json!({
                    "id": call.id.clone().unwrap_or_default(),
                    "name": call.name.clone(),
                    "arguments": call.arguments.clone()
                })
            }).collect::<Vec<_>>()
        });

        // Store as JSON string in the message
        conversation.push(Message::assistant(
            serde_json::to_string(&message_with_tools).unwrap_or_else(|_| content.to_string()),
        ));
    } else {
        // No tool calls, just store the content directly
        conversation.push(Message::assistant(content.to_string()));
    }
}

fn get_next_completion_options(
    loop_count: usize,
    max_loops: usize,
    base_options: &CompletionOptions,
) -> CompletionOptions {
    if loop_count >= max_loops - 1 {
        // On the last loop, request a final summary with no further tool calls
        CompletionOptions {
            require_tool_use: false,
            json_schema: Some(
                r#"{
                    "type": "object",
                    "properties": {
                        "finalSummary": {
                            "type": "string",
                            "description": "Final comprehensive summary of findings and results"
                        }
                    },
                    "required": ["finalSummary"]
                }"#
                .to_string(),
            ),
            ..base_options.clone()
        }
    } else {
        // For intermediate calls, continue with normal options
        base_options.clone()
    }
}

fn extract_content_from_response(content: &str) -> String {
    if content.trim().starts_with('{') && content.trim().ends_with('}') {
        // Try to parse as JSON to extract the finalSummary if available
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(summary) = json.get("finalSummary").and_then(|s| s.as_str()) {
                return summary.to_string();
            }
        }
    }

    content.to_string()
}

async fn send_error_message(sender: &Option<mpsc::Sender<String>>, message: &str) {
    if let Some(sender) = sender {
        let _ = sender.send(format!("[error] {}", message)).await;
    }
}

async fn execute_tool_with_preview(
    tool_call: &AgentToolCall,
    call: &ApiToolCall,
    progress_sender: &Option<mpsc::Sender<String>>,
) -> String {
    // Check if tool needs diff preview
    let needs_diff_preview = matches!(call.name.as_str(), "Edit" | "Replace");

    let result = if needs_diff_preview {
        // Handle file modification tools with diff preview
        match tool_call {
            AgentToolCall::Edit(params) => {
                use crate::tools::fs::file_ops::FileOps;
                use std::path::PathBuf;

                // Generate diff without making changes
                let path = PathBuf::from(&params.file_path);
                match FileOps::generate_edit_diff(
                    &path,
                    &params.old_string,
                    &params.new_string,
                    params.expected_replacements,
                ) {
                    Ok((_, diff)) => {
                        // Send diff as progress message
                        if let Some(sender) = progress_sender {
                            let _ = sender.send(diff.clone()).await;
                        }
                        // Execute the tool
                        tool_call.execute()
                    }
                    Err(e) => Err(e),
                }
            }
            AgentToolCall::Replace(params) => {
                use crate::tools::fs::file_ops::FileOps;
                use std::path::PathBuf;

                // Generate diff without making changes
                let path = PathBuf::from(&params.file_path);
                match FileOps::generate_write_diff(&path, &params.content) {
                    Ok((diff, _)) => {
                        // Send diff as progress message
                        if let Some(sender) = progress_sender {
                            let _ = sender.send(diff.clone()).await;
                        }
                        // Execute the tool
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
    };

    match result {
        Ok(output) => output,
        Err(e) => format!("ERROR EXECUTING TOOL: {}", e),
    }
}

fn parse_tool_call(name: &str, args: &Value) -> Result<AgentToolCall> {
    match name {
        "Read" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Read parameters")?;
            Ok(AgentToolCall::Read(params))
        }
        "Glob" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Glob parameters")?;
            Ok(AgentToolCall::Glob(params))
        }
        "Grep" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Grep parameters")?;
            Ok(AgentToolCall::Grep(params))
        }
        "LS" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse LS parameters")?;
            Ok(AgentToolCall::LS(params))
        }
        "Edit" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Edit parameters")?;
            Ok(AgentToolCall::Edit(params))
        }
        "Replace" => {
            let params = serde_json::from_value(args.clone())
                .context("Failed to parse Replace parameters")?;
            Ok(AgentToolCall::Replace(params))
        }
        "Bash" => {
            let params =
                serde_json::from_value(args.clone()).context("Failed to parse Bash parameters")?;
            Ok(AgentToolCall::Bash(params))
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}
