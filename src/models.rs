use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub file_name: String,
    pub size_gb: f32,
    pub description: String,
    pub primary_url: String,
    pub fallback_url: String,
    pub recommended_for: String,
    pub n_gpu_layers: usize,
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
        // Cloud models with agent capabilities
        ModelConfig {
            name: "Claude 3.7 Sonnet".into(),
            file_name: "claude-3-7-sonnet-20250219".into(),
            size_gb: 0.0, // Cloud model
            description: "Latest Anthropic Claude with advanced code capabilities".into(),
            primary_url: "".into(), // No download needed
            fallback_url: "".into(),
            recommended_for: "Professional code tasks, requires ANTHROPIC_API_KEY".into(),
            n_gpu_layers: 0, // Cloud model
            agentic_capabilities: Some(vec![
                AgentCapability::FileSearch,
                AgentCapability::CodeExecution,
                AgentCapability::FileEdit,
                AgentCapability::CodeCompletion,
                AgentCapability::CodeExplanation,
                AgentCapability::RepositoryNavigation,
            ]),
        },
        ModelConfig {
            name: "GPT-4o".into(),
            file_name: "gpt-4o".into(),
            size_gb: 0.0, // Cloud model
            description: "OpenAI GPT-4o with agent capabilities".into(),
            primary_url: "".into(), // No download needed
            fallback_url: "".into(),
            recommended_for: "Production code tasks, requires OPENAI_API_KEY".into(),
            n_gpu_layers: 0, // Cloud model
            agentic_capabilities: Some(vec![
                AgentCapability::FileSearch,
                AgentCapability::CodeExecution,
                AgentCapability::FileEdit,
                AgentCapability::CodeCompletion,
                AgentCapability::CodeExplanation,
                AgentCapability::RepositoryNavigation,
            ]),
        },
        // Local models
        ModelConfig {
            name: "QwQ-32B-Q4_K_M".into(),
            file_name: "QwQ-32B-Q4_K_M.gguf".into(),
            size_gb: 19.9,
            description: "Great for coding on M-series Macs".into(),
            primary_url: "https://huggingface.co/unsloth/QwQ-32B-GGUF/resolve/main/QwQ-32B-Q4_K_M.gguf".into(),
            fallback_url: "https://huggingface.co/unsloth/QwQ-32B-GGUF/resolve/main/QwQ-32B-Q4_K_M.gguf".into(),
            recommended_for: "M1/M2/M3/M4 Macs with 16GB+ RAM".into(),
            n_gpu_layers: 48, // High for M-series Macs (adjust if needed)
            agentic_capabilities: None,
        },
        ModelConfig {
            name: "CodeLlama-34B-GGUF".into(),
            file_name: "codellama-34b.Q2_K.gguf".into(),
            size_gb: 14.2,
            description: "Great for coding on M-series Macs".into(),
            primary_url: "https://huggingface.co/TheBloke/CodeLlama-34B-GGUF/resolve/main/codellama-34b.Q2_K.gguf".into(),
            fallback_url: "https://huggingface.co/TheBloke/CodeLlama-34B-GGUF/resolve/main/codellama-34b.Q2_K.gguf".into(),
            recommended_for: "M1/M2/M3/M4 Macs with 16GB+ RAM".into(),
            n_gpu_layers: 48, // High for M-series Macs (adjust if needed)
            agentic_capabilities: None,
        },
        ModelConfig {
            name: "TinyLlama-1.1B".into(),
            file_name: "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            size_gb: 0.65,
            description: "Very small, fast for testing".into(),
            primary_url: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            fallback_url: "https://huggingface.co/api/models/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            recommended_for: "Testing, low-resource systems".into(),
            n_gpu_layers: 1, // Use fewer GPU layers for this tiny model
            agentic_capabilities: None,
        },
    ]
}
