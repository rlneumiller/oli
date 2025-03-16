use crate::apis::api_client::{ApiClient, CompletionOptions, Message, ToolCall, ToolResult};
use crate::errors::AppError;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::env;

// Anthropic API models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_use: Option<Vec<AnthropicToolUse>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text { text: String },
    // Adding JSON variant for future use
    Json { json: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicToolUse {
    id: Option<String>,
    name: String,
    input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicTool {
    name: String,
    description: Option<String>,
    input_schema: AnthropicToolSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicToolSchema {
    #[serde(rename = "type")]
    type_field: String,
    properties: serde_json::Map<String, Value>,
    required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<AnthropicResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_use: Option<Vec<AnthropicToolUse>>,
}

pub struct AnthropicClient {
    client: ReqwestClient,
    model: String,
    api_base: String,
}

impl AnthropicClient {
    pub fn new(model: Option<String>) -> Result<Self> {
        // Try to get API key from environment
        let api_key = env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;

        // Create new client with appropriate headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let client = ReqwestClient::builder().default_headers(headers).build()?;

        // Default to Claude 3.7 Sonnet as the latest model with tooling capabilities
        let model = model.unwrap_or_else(|| "claude-3-sonnet-20240229".to_string());

        Ok(Self {
            client,
            model,
            api_base: "https://api.anthropic.com/v1/messages".to_string(),
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<AnthropicMessage> {
        messages
            .into_iter()
            .map(|msg| AnthropicMessage {
                role: msg.role,
                content: vec![AnthropicContent::Text { text: msg.content }],
                tool_use: None,
            })
            .collect()
    }

    fn convert_tool_definitions(
        &self,
        tools: Vec<crate::apis::api_client::ToolDefinition>,
    ) -> Vec<AnthropicTool> {
        tools
            .into_iter()
            .map(|tool| {
                // Extract required fields from parameters if available
                let required: Vec<String> = if let Value::Object(map) = &tool.parameters {
                    if let Some(Value::Array(req)) = map.get("required") {
                        req.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                AnthropicTool {
                    name: tool.name,
                    description: Some(tool.description),
                    input_schema: AnthropicToolSchema {
                        type_field: "object".to_string(),
                        properties: match tool.parameters {
                            Value::Object(map) => map,
                            _ => serde_json::Map::new(),
                        },
                        required,
                    },
                }
            })
            .collect()
    }
}

#[async_trait]
impl ApiClient for AnthropicClient {
    async fn complete(&self, messages: Vec<Message>, options: CompletionOptions) -> Result<String> {
        let converted_messages = self.convert_messages(messages);

        let max_tokens = options.max_tokens.unwrap_or(2048) as usize;

        let mut request = AnthropicRequest {
            model: self.model.clone(),
            messages: converted_messages,
            max_tokens,
            temperature: options.temperature,
            top_p: options.top_p,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        // Add structured output format if specified in options
        if let Some(json_schema) = &options.json_schema {
            request.response_format = Some(AnthropicResponseFormat {
                format_type: "json".to_string(),
                schema: serde_json::from_str(json_schema).ok(),
            });
        }

        let response = self
            .client
            .post(&self.api_base)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("Failed to send request to Anthropic: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "Anthropic API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::Other(format!("Failed to parse Anthropic response: {}", e)))?;

        // Extract content from response
        let content = anthropic_response
            .content
            .into_iter()
            .map(|content| match content {
                AnthropicContent::Text { text } => text,
                AnthropicContent::Json { json } => json.to_string(),
            })
            .next()
            .ok_or_else(|| AppError::Other("No content in Anthropic response".to_string()))?;

        Ok(content)
    }

    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)> {
        let mut converted_messages = self.convert_messages(messages);

        // Add tool results if they exist
        if let Some(results) = tool_results {
            // For each tool result, we need to add corresponding messages
            for result in results {
                // Create a tool use message (from assistant)
                let tool_use_msg = AnthropicMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    tool_use: Some(vec![AnthropicToolUse {
                        id: Some(result.tool_call_id.clone()),
                        name: "tool".to_string(), // We don't have the original name
                        input: json!({}),         // We don't need the input for this
                    }]),
                };

                // Create a tool result message (from user)
                let tool_result_msg = AnthropicMessage {
                    role: "user".to_string(),
                    content: vec![AnthropicContent::Text {
                        text: format!("Tool result: {}", result.output),
                    }],
                    tool_use: None,
                };

                // Add both messages to the conversation
                converted_messages.push(tool_use_msg);
                converted_messages.push(tool_result_msg);
            }
        }

        let max_tokens = options.max_tokens.unwrap_or(2048) as usize;

        let mut request = AnthropicRequest {
            model: self.model.clone(),
            messages: converted_messages,
            max_tokens,
            temperature: options.temperature,
            top_p: options.top_p,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        // Add structured output format if specified in options
        if let Some(json_schema) = &options.json_schema {
            request.response_format = Some(AnthropicResponseFormat {
                format_type: "json".to_string(),
                schema: serde_json::from_str(json_schema).ok(),
            });
        }

        // Add tools if they exist
        if let Some(tools) = options.tools {
            let converted_tools = self.convert_tool_definitions(tools);
            request.tools = Some(converted_tools);

            // Set tool choice based on option
            request.tool_choice = Some(if options.require_tool_use {
                "required".to_string()
            } else {
                "auto".to_string()
            });
        }

        let response = self
            .client
            .post(&self.api_base)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("Failed to send request to Anthropic: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "Anthropic API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::Other(format!("Failed to parse Anthropic response: {}", e)))?;

        // Extract content from response
        let content = anthropic_response
            .content
            .into_iter()
            .map(|content| match content {
                AnthropicContent::Text { text } => text,
                AnthropicContent::Json { json } => json.to_string(),
            })
            .next()
            .unwrap_or_default();

        // Extract tool calls if any
        let tool_calls = if let Some(tools) = anthropic_response.tool_use {
            if tools.is_empty() {
                None
            } else {
                let calls = tools
                    .into_iter()
                    .map(|tool| crate::apis::api_client::ToolCall {
                        name: tool.name,
                        arguments: tool.input,
                    })
                    .collect::<Vec<_>>();

                Some(calls)
            }
        } else {
            None
        };

        Ok((content, tool_calls))
    }
}
