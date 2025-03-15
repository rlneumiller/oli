use anyhow::Result;
use llama_cpp::{
    standard_sampler::StandardSampler, LlamaModel, LlamaParams, LlamaSession, SessionParams,
};

pub struct ModelSession {
    session: LlamaSession,
}

impl ModelSession {
    pub fn new(model_path: &std::path::Path, n_gpu_layers: usize) -> Result<Self> {
        // Configure model parameters for Metal acceleration
        let model_params = LlamaParams {
            n_gpu_layers: n_gpu_layers.try_into().unwrap(),
            ..Default::default()
        };

        // Load model with Metal support
        let model = LlamaModel::load_from_file(model_path, model_params)?;

        // Create session with optimized parameters
        let session_params = SessionParams {
            n_ctx: 4096, // Context window size
            ..Default::default()
        };
        let session = model.create_session(session_params)?;

        Ok(Self { session })
    }

    pub fn generate(&mut self, prompt: &str) -> Result<String> {
        // Create a system prompt that makes the model good at code assistance
        let system_prompt = r#"You are an expert programming assistant that helps write, explain, and debug code.
- Focus on providing concise, practical solutions
- Explain key concepts clearly but concisely 
- When explaining code, break down complex parts step by step
- Use Markdown for code formatting - always use appropriate language tags
- Error messages should be analyzed in detail
- Prioritize best practices, readability, and maintainability
- Focus on performance, security, and edge cases when relevant
"#;

        let formatted_prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n\
            <|im_start|>user\n{}<|im_end|>\n\
            <|im_start|>assistant\n",
            system_prompt, prompt
        );

        // Feed prompt to the model
        self.session.advance_context(&formatted_prompt)?;

        // Generate response with proper error handling
        let mut output = String::new();
        let completions = self
            .session
            .start_completing_with(StandardSampler::default(), 2048)? // Longer output limit
            .into_strings();

        for token in completions {
            output.push_str(&token);
        }

        Ok(output.replace("<|im_end|>", "").trim().to_string())
    }
}
