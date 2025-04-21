use crate::apis::api_client::{ApiClient, CompletionOptions, Message, ToolCall, ToolResult};
use crate::app::logger::{format_log_with_color, LogLevel};
use crate::errors::AppError;
use crate::models::GEMINI_MODEL_NAME;
use anyhow::{Context, Result};
use async_trait::async_trait;
use rand;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client as ReqwestClient;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::env;
use std::time::Duration;

// Gemini API models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiContent {
    Text {
        text: String,
    },
    FunctionCall {
        function_call: GeminiFunctionCall,
    },
    FunctionResponse {
        function_response: GeminiFunctionResponse,
    },
    // Add more flexible fallback variant for unexpected response formats
    Other(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    // The Gemini API uses 'args' field for function arguments
    args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFunctionResponse {
    name: String,
    response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFunction {
    name: String,
    description: Option<String>,
    parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiTool {
    // Note Gemini API expects 'functionDeclarations' (camelCase), not 'function_declarations'
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiCandidate {
    content: GeminiMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_ratings: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiUsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}

pub struct GeminiClient {
    client: ReqwestClient,
    #[allow(dead_code)] // Keep the model field for consistency with other API clients
    model: String,
    api_base: String,
}

impl GeminiClient {
    // Helper function to send a request with retry logic for overload errors
    async fn send_request_with_retry<T: serde::Serialize + Clone>(
        &self,
        request: &T,
    ) -> Result<Response> {
        // Implement retry logic with exponential backoff
        let mut retries = 0;
        let max_retries = 3; // Maximum number of retries
        let mut delay_ms = 1000; // Start with 1 second delay

        loop {
            let result = self.client.post(&self.api_base).json(request).send().await;

            match result {
                Ok(resp) => {
                    // If response is 429 (rate limit) or 503 (overloaded), retry
                    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || resp.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE
                    {
                        if retries >= max_retries {
                            // Return the last error response if max retries reached
                            return Ok(resp);
                        }

                        // Extract retry-after header if available before cloning for the error body
                        let retry_after = resp
                            .headers()
                            .get("retry-after")
                            .and_then(|val| val.to_str().ok())
                            .and_then(|val| val.parse::<u64>().ok())
                            .unwrap_or(delay_ms);

                        // Clone the response for logging
                        let error_body = resp.text().await.unwrap_or_default();
                        eprintln!(
                            "{}",
                            format_log_with_color(
                                LogLevel::Warning,
                                &format!("Gemini API rate limited or overloaded: {}", error_body)
                            )
                        );

                        // Exponential backoff with jitter
                        let jitter = rand::random::<u64>() % 500;
                        let sleep_duration = Duration::from_millis(retry_after + jitter);

                        // Sleep and retry
                        tokio::time::sleep(sleep_duration).await;

                        // Increase delay for next retry
                        delay_ms = (delay_ms * 2).min(10000); // Cap at 10 seconds
                        retries += 1;
                        continue;
                    }

                    // For other status codes, return the response
                    return Ok(resp);
                }
                Err(e) => {
                    // For network errors, also use retry logic
                    if retries >= max_retries {
                        return Err(AppError::NetworkError(format!(
                            "Failed to send request to Gemini after {} retries: {}",
                            retries, e
                        ))
                        .into());
                    }

                    // Exponential backoff with jitter
                    let jitter = rand::random::<u64>() % 500;
                    let sleep_duration = Duration::from_millis(delay_ms + jitter);
                    tokio::time::sleep(sleep_duration).await;

                    // Increase delay for next retry
                    delay_ms = (delay_ms * 2).min(10000); // Cap at 10 seconds
                    retries += 1;
                }
            }
        }
    }

    pub fn new(model: Option<String>) -> Result<Self> {
        // Try to get API key from environment
        let api_key =
            env::var("GEMINI_API_KEY").context("GEMINI_API_KEY environment variable not set")?;

        Self::with_api_key(api_key, model)
    }

    pub fn with_api_key(api_key: String, model: Option<String>) -> Result<Self> {
        // Create new client with content-type header only
        // Note: Gemini API uses API key in URL query param, not as Bearer token
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = ReqwestClient::builder().default_headers(headers).build()?;

        // Default to the centrally defined Gemini model name
        let model = model.unwrap_or_else(|| GEMINI_MODEL_NAME.to_string());

        // API base URL with model and API key as query parameter, using v1beta endpoint
        let api_base = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}",
            model = model,
            api_key = api_key
        );

        eprintln!(
            "{}",
            format_log_with_color(LogLevel::Info, "Using Gemini API v1beta endpoint")
        );

        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Info,
                &format!("Configured Gemini API with model: {}", model)
            )
        );

        Ok(Self {
            client,
            model,
            api_base,
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<GeminiMessage> {
        let mut gemini_messages = Vec::new();
        let mut current_role = String::new();
        let mut current_parts = Vec::new();

        for msg in messages {
            let role = match msg.role.as_str() {
                "system" => "user", // Gemini treats system messages as user messages
                "user" => "user",
                "assistant" => "model",
                _ => "user", // Default to user for unknown roles
            };

            // If role changes, add the previous message and start a new one
            if !current_role.is_empty() && current_role != role {
                gemini_messages.push(GeminiMessage {
                    role: current_role,
                    parts: current_parts,
                });
                current_parts = Vec::new();
            }

            // Update current role
            current_role = role.to_string();

            // Add content
            current_parts.push(GeminiContent::Text {
                text: msg.content.clone(),
            });
        }

        // Add the final message if there's anything
        if !current_role.is_empty() && !current_parts.is_empty() {
            gemini_messages.push(GeminiMessage {
                role: current_role,
                parts: current_parts,
            });
        }

        gemini_messages
    }

    fn convert_tool_definitions(
        &self,
        tools: Vec<crate::apis::api_client::ToolDefinition>,
    ) -> Vec<GeminiTool> {
        let function_declarations = tools
            .into_iter()
            .map(|tool| GeminiFunction {
                name: tool.name,
                description: Some(tool.description),
                parameters: tool.parameters,
            })
            .collect();

        vec![GeminiTool {
            function_declarations,
        }]
    }

    fn extract_tool_calls(&self, response: &GeminiResponse) -> Option<Vec<ToolCall>> {
        if response.candidates.is_empty() {
            return None;
        }

        let candidate = &response.candidates[0];
        let mut tool_calls = Vec::new();

        for part in &candidate.content.parts {
            match part {
                GeminiContent::FunctionCall { function_call } => {
                    // Clone values for logging
                    let name = function_call.name.clone();
                    let args = function_call.args.clone();

                    // Each function_call in Gemini becomes a ToolCall in our system
                    tool_calls.push(ToolCall {
                        id: Some(format!("gemini-call-{}", rand::random::<u64>())),
                        name: name.clone(),
                        arguments: args.clone(),
                    });

                    // Log the function call for debugging
                    eprintln!(
                        "{}",
                        format_log_with_color(
                            LogLevel::Info,
                            &format!(
                                "Found function call: {} with args: {}",
                                name,
                                serde_json::to_string(&args).unwrap_or_default()
                            )
                        )
                    );
                }
                GeminiContent::Other(value) => {
                    // Try to extract function call from the "Other" variant - check both camelCase and snake_case
                    if let Some(fc_value) = value
                        .get("functionCall")
                        .or_else(|| value.get("function_call"))
                    {
                        // Extract function call from JSON value
                        if let Some(name) = fc_value.get("name").and_then(|n| n.as_str()) {
                            let args = fc_value.get("args").cloned().unwrap_or(json!({}));

                            // Clone args for logging before moving it into the ToolCall
                            let args_for_log = args.clone();

                            tool_calls.push(ToolCall {
                                id: Some(format!("gemini-call-{}", rand::random::<u64>())),
                                name: name.to_string(),
                                arguments: args,
                            });

                            // Log the extracted function call using the cloned args
                            eprintln!(
                                "{}",
                                format_log_with_color(
                                    LogLevel::Info,
                                    &format!("Extracted function call from Other variant: {} with args: {}",
                                        name,
                                        serde_json::to_string(&args_for_log).unwrap_or_default()
                                    )
                                )
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        }
    }

    fn add_tool_results(&self, messages: &mut Vec<GeminiMessage>, tool_results: Vec<ToolResult>) {
        // For each tool result, add both the function call and its response
        for result in tool_results {
            // Create a synthetic function call message from the model
            let function_call_message = GeminiMessage {
                role: "model".to_string(),
                parts: vec![GeminiContent::FunctionCall {
                    function_call: GeminiFunctionCall {
                        name: "function".to_string(), // Generic name as we might not know the exact one
                        args: json!({}), // Empty args as we don't have the original call
                    },
                }],
            };

            // Create a function response message
            let function_response_message = GeminiMessage {
                role: "user".to_string(),
                parts: vec![GeminiContent::FunctionResponse {
                    function_response: GeminiFunctionResponse {
                        name: "function".to_string(), // Generic name
                        response: json!({
                            "content": result.output,
                            "tool_call_id": result.tool_call_id
                        }),
                    },
                }],
            };

            // Add both messages to maintain the conversation flow
            messages.push(function_call_message);
            messages.push(function_response_message);
        }
    }

    fn extract_text_content(&self, response: &GeminiResponse) -> Result<String> {
        if response.candidates.is_empty() {
            return Err(AppError::LLMError("No response candidates returned".to_string()).into());
        }

        let candidate = &response.candidates[0];
        let mut text_content = String::new();

        // Log response structure for debugging
        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!(
                    "Gemini response parts count: {}",
                    candidate.content.parts.len()
                )
            )
        );

        // Try to extract text content from parts
        for part in &candidate.content.parts {
            match part {
                GeminiContent::Text { text } => {
                    text_content = text.clone();
                    break;
                }
                GeminiContent::Other(value) => {
                    // Try to extract text from "text" field if it exists
                    if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
                        text_content = text.to_string();
                        break;
                    }
                }
                _ => {}
            }
        }

        if text_content.is_empty() {
            // Check if this is a pure function call response (no text content)
            let has_function_call = candidate.content.parts.iter().any(|part| match part {
                GeminiContent::FunctionCall { .. } => true,
                GeminiContent::Other(value) => value
                    .get("functionCall")
                    .or_else(|| value.get("function_call"))
                    .is_some(),
                _ => false,
            });

            if has_function_call {
                // Return empty string if we found function calls but no text
                eprintln!(
                    "{}",
                    format_log_with_color(
                        LogLevel::Info,
                        "Found function call(s) but no text content in Gemini response"
                    )
                );
                return Ok(String::new());
            }

            // Log the full response if no text is found
            if let Ok(response_str) = serde_json::to_string_pretty(response) {
                eprintln!(
                    "{}",
                    format_log_with_color(
                        LogLevel::Debug,
                        &format!("Full Gemini response: {}", response_str)
                    )
                );
            }

            // Return an error if no text content or function calls found
            return Err(AppError::LLMError(
                "No text content or function calls in Gemini response".to_string(),
            )
            .into());
        }

        Ok(text_content)
    }
}

#[async_trait]
impl ApiClient for GeminiClient {
    async fn complete(&self, messages: Vec<Message>, options: CompletionOptions) -> Result<String> {
        // Convert messages to Gemini format
        let contents = self.convert_messages(messages);

        let max_tokens = options.max_tokens.unwrap_or(2048);

        // Create generation config
        let generation_config = GeminiGenerationConfig {
            temperature: options.temperature,
            top_p: options.top_p,
            max_output_tokens: Some(max_tokens),
            response_mime_type: if options.json_schema.is_some() {
                Some("application/json".to_string())
            } else {
                None
            },
        };

        let request = GeminiRequest {
            contents,
            tools: None,
            generation_config: Some(generation_config),
        };

        // Send request with retry logic
        let response = self.send_request_with_retry(&request).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "Gemini API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        // Get the response as a string for debugging
        let response_text = response.text().await.map_err(|e| {
            let error_msg = format!("Failed to get response text: {}", e);
            eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
            AppError::NetworkError(error_msg)
        })?;

        // Log response details
        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!(
                    "Gemini API response received: {} bytes",
                    response_text.len()
                )
            )
        );

        // Parse the response
        let gemini_response: GeminiResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                let error_msg = format!("Failed to parse Gemini response: {}", e);
                eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
                AppError::Other(error_msg)
            })?;

        // Extract text content
        let content = self.extract_text_content(&gemini_response)?;

        Ok(content)
    }

    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)> {
        // Convert messages to Gemini format
        let mut contents = self.convert_messages(messages);

        // Add tool results if they exist
        if let Some(results) = tool_results {
            self.add_tool_results(&mut contents, results);
        }

        let max_tokens = options.max_tokens.unwrap_or(2048);

        // Create generation config
        let generation_config = GeminiGenerationConfig {
            temperature: options.temperature,
            top_p: options.top_p,
            max_output_tokens: Some(max_tokens),
            response_mime_type: if options.json_schema.is_some() {
                Some("application/json".to_string())
            } else {
                None
            },
        };

        // Create the request
        let mut request = GeminiRequest {
            contents,
            generation_config: Some(generation_config),
            tools: None,
        };

        // Add tools if specified
        if let Some(tools) = options.tools {
            request.tools = Some(self.convert_tool_definitions(tools));
        }

        // Send request with retry logic
        let response = self.send_request_with_retry(&request).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::NetworkError(format!(
                "Gemini API error: {} - {}",
                status, error_text
            ))
            .into());
        }

        // Get the response as a string for debugging
        let response_text = response.text().await.map_err(|e| {
            let error_msg = format!("Failed to get response text: {}", e);
            eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
            AppError::NetworkError(error_msg)
        })?;

        // Log response details
        eprintln!(
            "{}",
            format_log_with_color(
                LogLevel::Debug,
                &format!(
                    "Gemini API response received: {} bytes",
                    response_text.len()
                )
            )
        );

        // Parse the response
        let gemini_response: GeminiResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                let error_msg = format!("Failed to parse Gemini response: {}", e);
                eprintln!("{}", format_log_with_color(LogLevel::Error, &error_msg));
                AppError::Other(error_msg)
            })?;

        // Extract text content (may be empty if only function calls)
        let content = self
            .extract_text_content(&gemini_response)
            .unwrap_or_default();

        // Extract tool calls
        let tool_calls = self.extract_tool_calls(&gemini_response);

        Ok((content, tool_calls))
    }
}
