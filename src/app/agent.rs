use crate::agent::core::LLMProvider;
use crate::tools::code::parser::CodeParser;
use anyhow::{Context, Result};
use std::path::Path;

pub trait AgentManager {
    fn setup_agent(&mut self) -> Result<()>;
    fn query_model(&mut self, prompt: &str) -> Result<String>;
    fn query_with_agent(&mut self, prompt: &str) -> Result<String>;

    /// Handle a parse_code command request with the given file path
    fn handle_parse_code_command(&mut self, file_path: &str) -> Result<String> {
        // Get current working directory
        let cwd = self
            .current_working_dir()
            .map(|p| p.to_string())
            .unwrap_or_else(|| ".".to_string());

        // Initialize parser
        let mut parser = CodeParser::new().context("Failed to initialize code parser")?;

        // Normalize file path - if it's not absolute, make it relative to current dir
        let path = if Path::new(file_path).is_absolute() {
            Path::new(file_path).to_path_buf()
        } else {
            Path::new(&cwd).join(file_path)
        };

        // Check if file exists
        if !path.exists() {
            return Ok(format!("File not found: {}", path.display()));
        }

        // Parse the file
        match parser.parse_file(&path) {
            Ok(ast) => {
                // Convert AST to structured text output
                let structured_output = serde_json::to_string_pretty(&ast)
                    .context("Failed to serialize AST to JSON")?;

                // Return formatted output with file info
                Ok(format!(
                    "# Code Structure Analysis for {}\n\n```json\n{}\n```",
                    path.display(),
                    structured_output
                ))
            }
            Err(err) => Ok(format!("Error parsing file {}: {}", path.display(), err)),
        }
    }

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
