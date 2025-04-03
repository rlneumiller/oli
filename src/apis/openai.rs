use crate::apis::api_client::{
    ApiClient, CompletionOptions, Message, ToolCall, ToolDefinition, ToolResult,
};
use crate::errors::AppError;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::env;

// OpenAI API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIResponseChoice {
    index: usize,
    message: OpenAIMessage,
    finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIResponseChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Value>,
}

pub struct OpenAIClient {
    client: ReqwestClient,
    model: String,
    api_base: String,
}

impl OpenAIClient {
    pub fn new(model: Option<String>) -> Result<Self> {
        // Try to get API key from environment
        let api_key =
            env::var("OPENAI_API_KEY").context("OPENAI_API_KEY environment variable not set")?;

        Self::with_api_key(api_key, model)
    }

    pub fn with_api_key(api_key: String, model: Option<String>) -> Result<Self> {
        // Create new client with appropriate headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );

        let client = ReqwestClient::builder().default_headers(headers).build()?;

        // Default to GPT-4o as the latest model with tooling capabilities
        let model = model.unwrap_or_else(|| "gpt-4o".to_string());

        Ok(Self {
            client,
            model,
            api_base: "https://api.openai.com/v1/chat/completions".to_string(),
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<OpenAIMessage> {
        messages
            .into_iter()
            .map(|msg| {
                // Convert standard messages
                OpenAIMessage {
                    role: msg.role,
                    content: Some(msg.content),
                    tool_calls: None,
                    tool_call_id: None,
                }
            })
            .collect()
    }

    fn convert_tool_definitions(&self, tools: Vec<ToolDefinition>) -> Vec<OpenAITool> {
        tools
            .into_iter()
            .map(|tool| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                },
            })
            .collect()
    }
}

#[async_trait]
impl ApiClient for OpenAIClient {
    async fn complete(&self, messages: Vec<Message>, options: CompletionOptions) -> Result<String> {
        let openai_messages = self.convert_messages(messages);

        let mut request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            top_p: options.top_p,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        // Add structured output format if specified in options
        if let Some(_json_schema) = &options.json_schema {
            request.response_format = Some(json!({
                "type": "json_object"
            }));
        }

        let response = self
            .client
            .post(&self.api_base)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("Failed to send request to OpenAI: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "OpenAI API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        // Parse response
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::NetworkError(format!("Failed to get response text: {}", e)))?;

        let openai_response: OpenAIResponse = serde_json::from_str(&response_text)
            .map_err(|e| AppError::Other(format!("Failed to parse OpenAI response: {}", e)))?;

        // Extract content from the first choice
        if let Some(first_choice) = openai_response.choices.first() {
            if let Some(content) = &first_choice.message.content {
                return Ok(content.clone());
            }
        }

        Err(AppError::LLMError("No content in OpenAI response".to_string()).into())
    }

    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)> {
        // Convert messages to OpenAI format
        let mut openai_messages = self.convert_messages(messages);

        // Track tool calls that need responses
        let mut pending_tool_calls = Vec::new();

        // First pass: identify all tool calls that need responses
        for msg in &openai_messages {
            if msg.role == "assistant" && msg.tool_calls.is_some() {
                if let Some(tool_calls) = &msg.tool_calls {
                    for call in tool_calls {
                        pending_tool_calls.push(call.id.clone());
                    }
                }
            }
        }

        // Second pass: remove tool call IDs that already have responses
        for msg in &openai_messages {
            if msg.role == "tool" && msg.tool_call_id.is_some() {
                if let Some(tool_call_id) = &msg.tool_call_id {
                    pending_tool_calls.retain(|id| id != tool_call_id);
                }
            }
        }

        // Add tool results for any pending tool calls
        if let Some(results) = &tool_results {
            let result_map: std::collections::HashMap<String, String> = results
                .iter()
                .map(|r| (r.tool_call_id.clone(), r.output.clone()))
                .collect();

            // Add responses for any pending tool calls
            for tool_id in &pending_tool_calls {
                if let Some(output) = result_map.get(tool_id) {
                    openai_messages.push(OpenAIMessage {
                        role: "tool".to_string(),
                        content: Some(output.clone()),
                        tool_calls: None,
                        tool_call_id: Some(tool_id.clone()),
                    });
                } else {
                    // For any tool call without a provided result, add a default response
                    // This is crucial for OpenAI - every tool call must have a response
                    openai_messages.push(OpenAIMessage {
                        role: "tool".to_string(),
                        content: Some(
                            "Tool execution completed without detailed results.".to_string(),
                        ),
                        tool_calls: None,
                        tool_call_id: Some(tool_id.clone()),
                    });
                }
            }
        } else if !pending_tool_calls.is_empty() {
            // If we have pending tool calls but no results were provided,
            // we need to add default responses for all pending tool calls
            for tool_id in &pending_tool_calls {
                openai_messages.push(OpenAIMessage {
                    role: "tool".to_string(),
                    content: Some("Tool execution completed without detailed results.".to_string()),
                    tool_calls: None,
                    tool_call_id: Some(tool_id.clone()),
                });
            }
        }

        let mut request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            top_p: options.top_p,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        // Add structured output format if specified in options
        if let Some(_json_schema) = &options.json_schema {
            request.response_format = Some(json!({
                "type": "json_object"
            }));

            // Ensure at least one message contains the word "json" when using json_object response format
            let has_json_keyword = request.messages.iter().any(|msg| {
                msg.content
                    .as_ref()
                    .is_some_and(|content| content.to_lowercase().contains("json"))
            });

            if !has_json_keyword && !request.messages.is_empty() {
                // Add "json" to the user's last message if it doesn't already contain it
                if let Some(last_user_msg) = request
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|msg| msg.role == "user")
                {
                    if let Some(content) = &mut last_user_msg.content {
                        *content = format!("{} (Please provide the response as JSON)", content);
                    }
                }
            }
        }

        // Add tools if they exist
        if let Some(tools) = options.tools {
            let converted_tools = self.convert_tool_definitions(tools);
            request.tools = Some(converted_tools);

            // Set tool_choice based on option
            request.tool_choice = if options.require_tool_use {
                Some("required".to_string())
            } else {
                Some("auto".to_string())
            };
        }

        let response = self
            .client
            .post(&self.api_base)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("Failed to send request to OpenAI: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "OpenAI API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        // Parse response
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::NetworkError(format!("Failed to get response text: {}", e)))?;

        let openai_response: OpenAIResponse = serde_json::from_str(&response_text)
            .map_err(|e| AppError::Other(format!("Failed to parse OpenAI response: {}", e)))?;

        // Extract content and tool calls from the first choice
        if let Some(first_choice) = openai_response.choices.first() {
            let content = first_choice.message.content.clone().unwrap_or_default();

            // Extract tool calls if present
            let tool_calls = if let Some(openai_tool_calls) = &first_choice.message.tool_calls {
                if openai_tool_calls.is_empty() {
                    None
                } else {
                    let calls = openai_tool_calls
                        .iter()
                        .map(|call| {
                            // Parse arguments as JSON
                            let arguments_result =
                                serde_json::from_str::<Value>(&call.function.arguments);
                            let arguments = match arguments_result {
                                Ok(args) => args,
                                Err(_) => json!({}),
                            };

                            // Create a tool call with OpenAI's required format
                            ToolCall {
                                id: Some(call.id.clone()), // Important for tool results later
                                name: call.function.name.clone(),
                                arguments,
                            }
                        })
                        .collect::<Vec<_>>();

                    if calls.is_empty() {
                        None
                    } else {
                        Some(calls)
                    }
                }
            } else {
                None
            };

            return Ok((content, tool_calls));
        }

        Ok((String::new(), None))
    }
}
