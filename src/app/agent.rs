use crate::agent::core::LLMProvider;
use anyhow::{Context, Result};
use std::path::Path;

pub trait AgentManager {
    fn setup_agent(&mut self) -> Result<()>;
    fn query_model(&mut self, prompt: &str) -> Result<String>;
    fn query_with_agent(&mut self, prompt: &str) -> Result<String>;


    /// Get the current working directory
    fn current_working_dir(&self) -> Option<&str>;
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
        // For Ollama models, return the file_name directly
        model_name if model_name.contains("Local") => {
            // The file_name field in ModelConfig already contains the correct Ollama model name
            // We need to get the ModelConfig from the available_models list
            let models = crate::models::get_available_models();
            let model_config = models.iter().find(|m| m.name == model_name);

            if let Some(config) = model_config {
                Some(config.file_name.clone())
            } else {
                Some("qwen2.5-coder:14b".to_string()) // Default model if we can't find it
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
        // For Ollama models, check if the model name contains "Local"
        model_name if model_name.contains("Local") => {
            // Ollama models don't require API keys, so we can always return the provider
            Some(LLMProvider::Ollama)
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
