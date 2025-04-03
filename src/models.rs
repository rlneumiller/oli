use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub file_name: String,
    pub description: String,
    pub recommended_for: String,
    pub supports_agent: bool,
}

impl ModelConfig {
    pub fn has_agent_support(&self) -> bool {
        self.supports_agent
    }
}

use crate::apis::ollama::OllamaClient;
use anyhow::Result;

pub fn get_available_models() -> Vec<ModelConfig> {
    // Start with just the API models
    let mut models = vec![
        // Claude 3.7 Sonnet - Anthropic model supporting tool use
        ModelConfig {
            name: "Claude 3.7 Sonnet".into(),
            file_name: "claude-3-7-sonnet-20250219".into(),
            description: "Latest Anthropic Claude with advanced code capabilities".into(),
            recommended_for: "Professional code tasks, requires ANTHROPIC_API_KEY".into(),
            supports_agent: true,
        },
        // GPT-4o - OpenAI model supporting tool use
        ModelConfig {
            name: "GPT-4o".into(),
            file_name: "gpt-4o".into(),
            description: "Latest OpenAI model with advanced tool use capabilities".into(),
            recommended_for: "Professional code tasks, requires OPENAI_API_KEY".into(),
            supports_agent: true,
        },
    ];

    // Try to fetch available models from Ollama
    if let Ok(ollama_models) = get_available_ollama_models() {
        // Add each available Ollama model to the list
        for model_info in ollama_models {
            // Create a description based on the model details
            let description = if let Some(details) = &model_info.details {
                if let Some(desc) = &details.description {
                    format!("{} - Running locally via Ollama", desc)
                } else {
                    format!("{} - Running locally via Ollama", model_info.name)
                }
            } else {
                format!("{} - Running locally via Ollama", model_info.name)
            };

            // Add the model to the list
            models.push(ModelConfig {
                name: format!("{} - Local", model_info.name),
                file_name: model_info.name.clone(),
                description,
                recommended_for: "Local code tasks, requires Ollama to be running".into(),
                supports_agent: true,
            });
        }
    }

    models
}

fn get_available_ollama_models() -> Result<Vec<crate::apis::ollama::OllamaModelInfo>> {
    // Try to get the list of models from Ollama in a non-async context
    // We'll use a short timeout to avoid blocking the UI if Ollama is not running

    // Create a runtime for the async call
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Try to list models with a timeout - if it fails, we just return an empty list
    match runtime.block_on(async {
        // Handle the client creation result explicitly rather than using ?
        match OllamaClient::new(None) {
            Ok(ollama_client) => {
                // Use a short timeout (2 seconds) to avoid hanging if Ollama is not running
                let models_future = ollama_client.list_models();
                tokio::time::timeout(std::time::Duration::from_secs(2), models_future).await
            }
            Err(e) => {
                // Return as a timeout error type to match the outer result type
                Ok(Err(anyhow::anyhow!(
                    "Failed to create Ollama client: {}",
                    e
                )))
            }
        }
    }) {
        Ok(Ok(models)) => Ok(models),
        Err(_) => {
            // Return empty list on timeout
            Ok(Vec::new())
        }
        Ok(Err(_)) => {
            // Return empty list on error
            Ok(Vec::new())
        }
    }
}
