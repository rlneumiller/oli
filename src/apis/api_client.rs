use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>, // Required for OpenAI to map tool results back to calls
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub json_schema: Option<String>,
    pub require_tool_use: bool,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(2048),
            tools: None,
            json_schema: None,
            require_tool_use: false,
        }
    }
}

// This trait cannot be made into a dyn trait because it has async methods
#[async_trait::async_trait]
pub trait ApiClient: Send + Sync {
    // Basic completion without tool usage
    #[allow(dead_code)]
    async fn complete(&self, messages: Vec<Message>, options: CompletionOptions) -> Result<String>;

    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)>;
}

// Instead of using a trait object, we'll use an enum to handle different providers
#[derive(Clone)]
pub enum ApiClientEnum {
    Anthropic(Arc<crate::apis::anthropic::AnthropicClient>),
    OpenAi(Arc<crate::apis::openai::OpenAIClient>),
}

impl ApiClientEnum {
    #[allow(dead_code)]
    pub async fn complete(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<String> {
        match self {
            Self::Anthropic(client) => client.complete(messages, options).await,
            Self::OpenAi(client) => client.complete(messages, options).await,
        }
    }

    pub async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<(String, Option<Vec<ToolCall>>)> {
        match self {
            Self::Anthropic(client) => {
                client
                    .complete_with_tools(messages, options, tool_results)
                    .await
            }
            Self::OpenAi(client) => {
                client
                    .complete_with_tools(messages, options, tool_results)
                    .await
            }
        }
    }
}

pub type DynApiClient = ApiClientEnum;
