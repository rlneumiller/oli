use crate::apis::api_client::{ApiClient, CompletionOptions, Message, ToolCall, ToolResult};
use crate::errors::AppError;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::time::Duration;

// Ollama API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaResponse {
    model: String,
    created_at: String,
    message: OllamaMessage,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eval_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eval_duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaListModelsResponse {
    models: Vec<OllamaModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<OllamaModelDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub format: Option<String>,
    pub families: Option<Vec<String>>,
    pub description: Option<String>,
}

pub struct OllamaClient {
    client: ReqwestClient,
    model: String,
    api_base: String,
}

impl OllamaClient {
    pub fn new(model: Option<String>) -> Result<Self> {
        // Default to qwen2.5-coder:14b model if None or empty string
        let model_name = match model {
            Some(m) if !m.trim().is_empty() => m,
            _ => "qwen2.5-coder:14b".to_string(),
        };

        Self::with_base_url(model_name, "http://localhost:11434".to_string())
    }

    pub fn with_base_url(model: String, api_base: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = ReqwestClient::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(300)) // 5 minutes timeout for operations
            .build()?;

        Ok(Self {
            client,
            model,
            api_base,
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<OllamaMessage> {
        messages
            .into_iter()
            .map(|msg| {
                // Convert standard messages to Ollama format
                OllamaMessage {
                    role: msg.role,
                    content: msg.content,
                }
            })
            .collect()
    }

    pub async fn list_models(&self) -> Result<Vec<OllamaModelInfo>> {
        let url = format!("{}/api/tags", self.api_base);

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                // Connection failed - likely Ollama is not running
                AppError::NetworkError(
                    "Failed to connect to Ollama server. Make sure 'ollama serve' is running."
                        .to_string(),
                )
            } else {
                AppError::NetworkError(format!("Failed to send request to Ollama: {}", e))
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "Ollama API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        // Parse response
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::NetworkError(format!("Failed to get response text: {}", e)))?;

        let models_response: OllamaListModelsResponse = serde_json::from_str(&response_text)
            .map_err(|e| AppError::Other(format!("Failed to parse Ollama response: {}", e)))?;

        Ok(models_response.models)
    }
}

#[async_trait]
impl ApiClient for OllamaClient {
    async fn complete(&self, messages: Vec<Message>, options: CompletionOptions) -> Result<String> {
        let ollama_messages = self.convert_messages(messages);

        // Make sure we have a valid model name
        let model_name = if self.model.is_empty() {
            "qwen2.5-coder:14b".to_string() // Fallback to the default model
        } else {
            self.model.clone()
        };

        let request = OllamaRequest {
            model: model_name,
            messages: ollama_messages,
            stream: false,
            temperature: options.temperature,
            top_p: options.top_p,
            options: None,
            format: if options.json_schema.is_some() {
                Some("json".to_string())
            } else {
                None
            },
        };

        let url = format!("{}/api/chat", self.api_base);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    // Connection failed - likely Ollama is not running
                    AppError::NetworkError(
                        "Failed to connect to Ollama server. Make sure 'ollama serve' is running."
                            .to_string(),
                    )
                } else {
                    AppError::NetworkError(format!("Failed to send request to Ollama: {}", e))
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "Ollama API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        // Parse response
        let response_text = response
            .text()
            .await
            .map_err(|e| AppError::NetworkError(format!("Failed to get response text: {}", e)))?;

        let ollama_response: OllamaResponse = serde_json::from_str(&response_text)
            .map_err(|e| AppError::Other(format!("Failed to parse Ollama response: {}", e)))?;

        Ok(ollama_response.message.content)
    }

    // Ollama does not natively support tools, so we need to implement a workaround
    // This implementation will embed the tool definitions in the prompt
    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)> {
        // Ensure we have a valid model
        if self.model.is_empty() {
            return Err(anyhow::anyhow!(
                "Model name is empty. Please select a valid Ollama model."
            ));
        }

        // If there are tool results, we should add them to the conversation
        let mut conversation_messages = messages.clone();

        // Add tool results if provided
        if let Some(results) = tool_results {
            for result in results {
                // Add the tool result as a system message
                conversation_messages.push(Message {
                    role: "system".to_string(),
                    content: format!("Tool result for {}: {}", result.tool_call_id, result.output),
                });
            }
        }

        // If tools are defined, add them to the system prompt
        let mut system_prompt = String::new();

        if let Some(tools) = &options.tools {
            system_prompt.push_str("Available tools:\n\n");

            for tool in tools {
                system_prompt.push_str(&format!("Tool: {}\n", tool.name));
                system_prompt.push_str(&format!("Description: {}\n", tool.description));
                system_prompt.push_str(&format!(
                    "Parameters: {}\n\n",
                    serde_json::to_string_pretty(&tool.parameters)?
                ));
            }

            system_prompt.push_str(
                "\nWhen you want to use a tool, respond with JSON in the following format:\n",
            );
            system_prompt.push_str("```json\n{\n  \"tool\": \"tool_name\",\n  \"args\": { ... parameters ... }\n}\n```\n");

            // Add the system message at the beginning
            conversation_messages.insert(
                0,
                Message {
                    role: "system".to_string(),
                    content: system_prompt,
                },
            );
        }

        // Use the regular complete function for the actual API call
        let response = self.complete(conversation_messages, options).await?;

        // Try to parse the response as a tool call if it looks like JSON
        if response.trim().starts_with('{') && response.trim().ends_with('}') {
            // Try to parse as JSON
            if let Ok(json_value) = serde_json::from_str::<Value>(&response) {
                // Check if it has the expected structure for a tool call
                if let (Some(tool_name), Some(tool_args)) = (
                    json_value.get("tool").and_then(|t| t.as_str()),
                    json_value.get("args"),
                ) {
                    let tool_call = ToolCall {
                        id: Some(format!("ollama-tool-{}", rand::random::<u64>())),
                        name: tool_name.to_string(),
                        arguments: tool_args.clone(),
                    };

                    return Ok((String::new(), Some(vec![tool_call])));
                }
            }
        }

        // If we can't parse as a tool call, just return the text
        Ok((response, None))
    }
}
