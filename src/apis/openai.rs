use crate::apis::api_client::{ApiClient, CompletionOptions, Message, ToolCall, ToolResult};
use crate::errors::AppError;
use anyhow::{Context, Result};
use serde_json::Value;
use std::env;

pub struct OpenAIClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(model: Option<String>) -> Result<Self> {
        // Try to get API key from environment
        let api_key =
            env::var("OPENAI_API_KEY").context("OPENAI_API_KEY environment variable not set")?;

        let client = reqwest::Client::new();
        let model = model.unwrap_or_else(|| "gpt-4o".to_string());

        Ok(Self {
            api_key,
            model,
            client,
        })
    }
}

#[async_trait::async_trait]
impl ApiClient for OpenAIClient {
    async fn complete(&self, messages: Vec<Message>, options: CompletionOptions) -> Result<String> {
        // Convert messages to OpenAI format
        let openai_messages: Vec<Value> = messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect();

        // Build request body
        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": openai_messages,
        });

        // Add optional parameters
        if let Some(temperature) = options.temperature {
            request_body["temperature"] = temperature.into();
        }
        if let Some(top_p) = options.top_p {
            request_body["top_p"] = top_p.into();
        }
        if let Some(max_tokens) = options.max_tokens {
            request_body["max_tokens"] = max_tokens.into();
        }

        // Make request to OpenAI API
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        // Parse response
        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        // Extract content from response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AppError::Other("Invalid response from OpenAI API".to_string()))?
            .to_string();

        Ok(content)
    }

    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)> {
        // Convert messages to OpenAI format
        let mut openai_messages: Vec<Value> = messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role,
                    "content": msg.content
                })
            })
            .collect();

        // Add tool results if they exist
        if let Some(results) = tool_results {
            for result in results {
                openai_messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": result.tool_call_id,
                    "content": result.output
                }));
            }
        }

        // Build request body
        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": openai_messages,
        });

        // Add optional parameters
        if let Some(temperature) = options.temperature {
            request_body["temperature"] = temperature.into();
        }
        if let Some(top_p) = options.top_p {
            request_body["top_p"] = top_p.into();
        }
        if let Some(max_tokens) = options.max_tokens {
            request_body["max_tokens"] = max_tokens.into();
        }

        // Add tools if they exist
        if let Some(tools) = &options.tools {
            let openai_tools: Vec<Value> = tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters
                        }
                    })
                })
                .collect();

            request_body["tools"] = openai_tools.into();
            request_body["tool_choice"] = "auto".into();
        }

        // Make request to OpenAI API
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        // Parse response
        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        // Extract content and tool calls from response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Parse tool calls if present
        let tool_calls = if let Some(tool_calls) =
            response_json["choices"][0]["message"]["tool_calls"].as_array()
        {
            if tool_calls.is_empty() {
                None
            } else {
                let calls = tool_calls
                    .iter()
                    .filter_map(|call| {
                        let name = call["function"]["name"].as_str()?;
                        let arguments = call["function"]["arguments"].clone();

                        Some(ToolCall {
                            name: name.to_string(),
                            arguments: serde_json::from_str(arguments.as_str()?)
                                .unwrap_or(Value::Null),
                        })
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

        Ok((content, tool_calls))
    }
}
