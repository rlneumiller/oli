use crate::apis::api_client::{
    ApiClient, CompletionOptions, Message, ToolCall, ToolDefinition, ToolResult,
};
use crate::app::logger::{format_log_with_color, LogLevel};
use crate::errors::AppError;
use anyhow::Result;
use async_trait::async_trait;
use rand;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::time::Duration;

// Ollama API Types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    #[serde(default)]
    #[serde(with = "content_string_or_object")]
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

// Custom serializer to handle content that might be a string or a complex object
mod content_string_or_object {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(content: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(content)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) => Ok(s),
            _ => Ok(value.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolCall {
    #[serde(default)]
    id: String,
    function: OllamaFunctionCall,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    tool_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    #[serde(with = "arguments_as_string_or_object")]
    arguments: String,
}

// Custom serde module for function arguments
mod arguments_as_string_or_object {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(arguments: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(arguments)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        match value {
            Value::String(s) => Ok(s),
            _ => {
                // If it's not a string, convert the JSON to a string
                let json_str = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
                Ok(json_str)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaFunction {
    name: String,
    description: String,
    parameters: Value,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
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
                    tool_calls: None,
                    tool_call_id: None,
                }
            })
            .collect()
    }

    fn convert_tool_definitions(&self, tools: Vec<ToolDefinition>) -> Vec<OllamaTool> {
        tools
            .into_iter()
            .map(|tool| OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.parameters,
                },
            })
            .collect()
    }

    pub async fn list_models(&self) -> Result<Vec<OllamaModelInfo>> {
        let url = format!("{}/api/tags", self.api_base);

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!("Listing Ollama models from: {}", url)
            )
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            let error_msg = if e.is_connect() {
                // Connection failed - likely Ollama is not running
                "Failed to connect to Ollama server. Make sure 'ollama serve' is running."
                    .to_string()
            } else {
                format!("Failed to send request to Ollama: {}", e)
            };
            eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
            AppError::NetworkError(error_msg)
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
        let response_text = response.text().await.map_err(|e| {
            let error_msg = format!("Failed to get response text: {}", e);
            eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
            AppError::NetworkError(error_msg)
        })?;

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!(
                    "Ollama API response received: {} bytes",
                    response_text.len()
                )
            )
        );

        let models_response: OllamaListModelsResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                let error_msg = format!("Failed to parse Ollama response: {}", e);
                eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
                AppError::LLMError(error_msg)
            })?;

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
            tools: None,
        };

        let url = format!("{}/api/chat", self.api_base);

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!("Sending request to Ollama API with model: {}", self.model)
            )
        );

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
        let response_text = response.text().await.map_err(|e| {
            let error_msg = format!("Failed to get response text: {}", e);
            eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
            AppError::NetworkError(error_msg)
        })?;

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!(
                    "Ollama API response received: {} bytes",
                    response_text.len()
                )
            )
        );

        // Try to parse as a direct response
        let ollama_response = match serde_json::from_str::<OllamaResponse>(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                // Log errors when parsing Ollama API response
                eprintln!(
                    "{}",
                    format_log_with_color(
                        LogLevel::Warning,
                        &format!("Failed to parse standard Ollama response: {}, attempting alternate parsing", e)
                    )
                );

                // Try to parse as a generic JSON value to extract what we need
                let json_value: Result<serde_json::Value, _> = serde_json::from_str(&response_text);
                if let Ok(value) = json_value {
                    if let Some(message) = value.get("message") {
                        let role = message
                            .get("role")
                            .and_then(|r| r.as_str())
                            .unwrap_or("assistant")
                            .to_string();

                        // Extract content, which might be a string or object
                        let content = match message.get("content") {
                            Some(c) if c.is_string() => c.as_str().unwrap_or("").to_string(),
                            Some(c) => c.to_string(),
                            None => "".to_string(),
                        };

                        // Construct a valid OllamaResponse with the extracted data
                        OllamaResponse {
                            model: value
                                .get("model")
                                .and_then(|m| m.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            created_at: value
                                .get("created_at")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string(),
                            message: OllamaMessage {
                                role,
                                content,
                                tool_calls: None,
                                tool_call_id: None,
                            },
                            done: value.get("done").and_then(|d| d.as_bool()).unwrap_or(true),
                            total_duration: None,
                            load_duration: None,
                            prompt_eval_duration: None,
                            eval_count: None,
                            eval_duration: None,
                        }
                    } else {
                        return Err(AppError::Other(format!(
                            "Failed to parse Ollama response: {}",
                            e
                        ))
                        .into());
                    }
                } else {
                    return Err(
                        AppError::Other(format!("Failed to parse Ollama response: {}", e)).into(),
                    );
                }
            }
        };

        Ok(ollama_response.message.content)
    }

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

        // Convert messages to Ollama format
        let mut ollama_messages = self.convert_messages(messages);

        // Add tool results if provided
        if let Some(results) = tool_results {
            for result in results {
                ollama_messages.push(OllamaMessage {
                    role: "tool".to_string(),
                    content: result.output,
                    tool_calls: None,
                    tool_call_id: Some(result.tool_call_id),
                });
            }
        }

        // Make sure we have a valid model name
        let model_name = if self.model.is_empty() {
            "qwen2.5-coder:14b".to_string() // Fallback to the default model
        } else {
            self.model.clone()
        };

        // Create the request payload
        let mut request = OllamaRequest {
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
            tools: None,
        };

        // Add tools if provided
        if let Some(tools) = options.tools {
            let converted_tools = self.convert_tool_definitions(tools);
            request.tools = Some(converted_tools);
        }

        let url = format!("{}/api/chat", self.api_base);

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!("Sending request to Ollama API with model: {}", self.model)
            )
        );

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
        let response_text = response.text().await.map_err(|e| {
            let error_msg = format!("Failed to get response text: {}", e);
            eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
            AppError::NetworkError(error_msg)
        })?;

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!(
                    "Ollama API response received: {} bytes",
                    response_text.len()
                )
            )
        );

        // Try to parse as a direct response
        let ollama_response = match serde_json::from_str::<OllamaResponse>(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                // Log errors when parsing Ollama API response
                eprintln!(
                    "{}",
                    format_log_with_color(
                        LogLevel::Warning,
                        &format!("Failed to parse standard Ollama response: {}, attempting alternate parsing", e)
                    )
                );

                // Try to parse as a generic JSON value to extract what we need
                let json_value: Result<serde_json::Value, _> = serde_json::from_str(&response_text);
                if let Ok(value) = json_value {
                    if let Some(message) = value.get("message") {
                        let role = message
                            .get("role")
                            .and_then(|r| r.as_str())
                            .unwrap_or("assistant")
                            .to_string();

                        // Extract content, which might be a string or object
                        let content = match message.get("content") {
                            Some(c) if c.is_string() => c.as_str().unwrap_or("").to_string(),
                            Some(c) => c.to_string(),
                            None => "".to_string(),
                        };

                        // Construct a valid OllamaResponse with the extracted data
                        OllamaResponse {
                            model: value
                                .get("model")
                                .and_then(|m| m.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            created_at: value
                                .get("created_at")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string(),
                            message: OllamaMessage {
                                role,
                                content,
                                tool_calls: None,
                                tool_call_id: None,
                            },
                            done: value.get("done").and_then(|d| d.as_bool()).unwrap_or(true),
                            total_duration: None,
                            load_duration: None,
                            prompt_eval_duration: None,
                            eval_count: None,
                            eval_duration: None,
                        }
                    } else {
                        return Err(AppError::Other(format!(
                            "Failed to parse Ollama response: {}",
                            e
                        ))
                        .into());
                    }
                } else {
                    return Err(
                        AppError::Other(format!("Failed to parse Ollama response: {}", e)).into(),
                    );
                }
            }
        };

        // Extract the content and tool calls from the response
        let content = ollama_response.message.content.clone();

        // Check for tool calls in the response
        if let Some(ollama_tool_calls) = ollama_response.message.tool_calls {
            if !ollama_tool_calls.is_empty() {
                let tool_calls = ollama_tool_calls
                    .iter()
                    .map(|call| {
                        // Parse arguments as JSON
                        let arguments_result =
                            serde_json::from_str::<Value>(&call.function.arguments);
                        let arguments = match arguments_result {
                            Ok(args) => args,
                            Err(_) => json!({}),
                        };

                        // Generate a random ID if one wasn't provided
                        let id = if call.id.is_empty() {
                            format!("ollama-tool-{}", rand::random::<u64>())
                        } else {
                            call.id.clone()
                        };

                        // Create a tool call
                        ToolCall {
                            id: Some(id),
                            name: call.function.name.clone(),
                            arguments,
                        }
                    })
                    .collect::<Vec<_>>();

                return Ok((String::new(), Some(tool_calls)));
            }
        }

        // Also try to check if the content itself contains a tool call in JSON format
        // This handles cases where Ollama doesn't properly format its tool_calls field
        // but still returns JSON in the content field that looks like a tool call
        let content_str = content.trim();
        if content_str.starts_with('{') && content_str.ends_with('}') {
            if let Ok(json_value) = serde_json::from_str::<Value>(content_str) {
                // Check for OpenAI style tool calls
                if let Some(tool_calls) = json_value.get("tool_calls").and_then(|tc| tc.as_array())
                {
                    if !tool_calls.is_empty() {
                        let calls = tool_calls
                            .iter()
                            .filter_map(|call| {
                                let id = call.get("id").and_then(|id| id.as_str()).unwrap_or("");
                                let function = call.get("function")?;
                                let name = function.get("name")?.as_str()?;
                                let arguments = function.get("arguments")?;

                                let args_str = arguments.as_str().unwrap_or("{}");
                                let args: Value =
                                    serde_json::from_str(args_str).unwrap_or(json!({}));

                                Some(ToolCall {
                                    id: Some(id.to_string()),
                                    name: name.to_string(),
                                    arguments: args,
                                })
                            })
                            .collect::<Vec<_>>();

                        if !calls.is_empty() {
                            return Ok((String::new(), Some(calls)));
                        }
                    }
                }

                // Check for the simpler/custom format that our old implementation expected
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

        // If no tool calls were found, just return the content
        Ok((content, None))
    }
}
