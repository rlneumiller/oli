use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub file_name: String,
    pub description: String,
    pub recommended_for: String,
    pub agentic_capabilities: Option<Vec<AgentCapability>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentCapability {
    FileSearch,
    CodeExecution,
    FileEdit,
    CodeCompletion,
    CodeExplanation,
    RepositoryNavigation,
}

impl ModelConfig {
    pub fn supports_capability(&self, capability: &AgentCapability) -> bool {
        match &self.agentic_capabilities {
            Some(capabilities) => capabilities.contains(capability),
            None => false,
        }
    }
}

pub fn get_available_models() -> Vec<ModelConfig> {
    vec![
        // Claude 3.7 Sonnet - Anthropic model supporting tool use
        ModelConfig {
            name: "Claude 3.7 Sonnet".into(),
            file_name: "claude-3-7-sonnet-20250219".into(),
            description: "Latest Anthropic Claude with advanced code capabilities".into(),
            recommended_for: "Professional code tasks, requires ANTHROPIC_API_KEY".into(),
            agentic_capabilities: Some(vec![
                AgentCapability::FileSearch,
                AgentCapability::CodeExecution,
                AgentCapability::FileEdit,
                AgentCapability::CodeCompletion,
                AgentCapability::CodeExplanation,
                AgentCapability::RepositoryNavigation,
            ]),
        },
        // GPT-4o - OpenAI model supporting tool use
        ModelConfig {
            name: "GPT-4o".into(),
            file_name: "gpt-4o".into(),
            description: "Latest OpenAI model with advanced tool use capabilities".into(),
            recommended_for: "Professional code tasks, requires OPENAI_API_KEY".into(),
            agentic_capabilities: Some(vec![
                AgentCapability::FileSearch,
                AgentCapability::CodeExecution,
                AgentCapability::FileEdit,
                AgentCapability::CodeCompletion,
                AgentCapability::CodeExplanation,
                AgentCapability::RepositoryNavigation,
            ]),
        },
    ]
}
