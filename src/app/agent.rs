use crate::agent::core::LLMProvider;
use anyhow::Result;

pub trait AgentManager {
    fn setup_agent(&mut self) -> Result<()>;
    fn query_model(&mut self, prompt: &str) -> Result<String>;
    fn query_with_agent(&mut self, prompt: &str) -> Result<String>;
}

// Helper function to determine the appropriate model for the agent based on provider
pub fn determine_agent_model(provider_name: &str, has_api_key: bool) -> Option<String> {
    match provider_name {
        "Claude 3.7 Sonnet" => {
            if has_api_key {
                Some("claude-3-7-sonnet-20250219".to_string())
            } else {
                None
            }
        }
        "GPT-4o" => {
            if has_api_key {
                Some("gpt-4o".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

// Helper function to determine provider from model name
pub fn determine_provider(
    model_name: &str,
    has_anthropic_key: bool,
    has_openai_key: bool,
) -> Option<LLMProvider> {
    match model_name {
        "GPT-4o" => {
            if has_openai_key {
                Some(LLMProvider::OpenAI)
            } else {
                None
            }
        }
        "Claude 3.7 Sonnet" => {
            if has_anthropic_key {
                Some(LLMProvider::Anthropic)
            } else {
                None
            }
        }
        _ => {
            // If using another model, pick available provider
            if has_anthropic_key {
                Some(LLMProvider::Anthropic)
            } else if has_openai_key {
                Some(LLMProvider::OpenAI)
            } else {
                None
            }
        }
    }
}
