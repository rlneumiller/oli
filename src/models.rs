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
}

pub fn get_available_models() -> Vec<ModelConfig> {
    vec![
        ModelConfig {
            name: "TinyLlama-1.1B".into(),
            file_name: "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            size_gb: 0.65,
            description: "Very small, fast for testing".into(),
            primary_url: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            fallback_url: "https://huggingface.co/api/models/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into(),
            recommended_for: "Testing, low-resource systems".into(),
            n_gpu_layers: 16,
        },
        ModelConfig {
            name: "Phi-2".into(),
            file_name: "phi-2.Q4_K_M.gguf".into(),
            size_gb: 1.64,
            description: "Microsoft's small but capable model".into(),
            primary_url: "https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf".into(),
            fallback_url: "https://huggingface.co/api/models/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf".into(),
            recommended_for: "Balanced systems".into(),
            n_gpu_layers: 24,
        },
        ModelConfig {
            name: "Mistral-7B-v0.2".into(),
            file_name: "mistral-7b-instruct-v0.2.Q4_K_M.gguf".into(),
            size_gb: 3.83,
            description: "Fast, balanced performance".into(),
            primary_url: "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF/resolve/main/mistral-7b-instruct-v0.2.Q4_K_M.gguf".into(),
            fallback_url: "https://huggingface.co/api/models/TheBloke/Mistral-7B-Instruct-v0.2-GGUF/files/mistral-7b-instruct-v0.2.Q4_K_M.gguf".into(),
            recommended_for: "All systems".into(),
            n_gpu_layers: 32,
        },
    ]
}
