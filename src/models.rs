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
            name: "QwQ-32B-Q4_K_M".into(),
            file_name: "QwQ-32B-Q4_K_M.gguf".into(),
            size_gb: 19.9,
            description: "Great for coding on M-series Macs".into(),
            primary_url: "https://huggingface.co/unsloth/QwQ-32B-GGUF/resolve/main/QwQ-32B-Q4_K_M.gguf".into(),
            fallback_url: "https://huggingface.co/unsloth/QwQ-32B-GGUF/resolve/main/QwQ-32B-Q4_K_M.gguf".into(),
            recommended_for: "M1/M2/M3/M4 Macs with 16GB+ RAM".into(),
            n_gpu_layers: 48, // High for M-series Macs (adjust if needed)
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
        },
    ]
}
