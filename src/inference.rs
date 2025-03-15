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
You help users complete coding tasks efficiently by understanding their requests.
Key characteristics:
- You provide practical, working solutions
- Explain code concepts clearly
- Break down complex problems step by step
- Format all code with appropriate language annotations
- Analyze error messages in detail
- Prioritize readability and maintainability
- Consider edge cases and security implications

When reading code, analyze it methodically before answering.
When writing code, provide complete, working solutions.
Be concise but thorough in your explanations.
"#;

        // Try different prompt formats to be compatible with various models
        let prompts_to_try = vec![
            // ChatML format (for Mistral, Llama, etc.)
            format!(
                "<|im_start|>system\n{}<|im_end|>\n\
                <|im_start|>user\n{}<|im_end|>\n\
                <|im_start|>assistant\n",
                system_prompt, prompt
            ),
            // Simple instruction format (for Phi-2 and some others)
            format!(
                "System: {}\n\nUser: {}\n\nAssistant: ",
                system_prompt, prompt
            ),
            // Direct prompt for simpler models
            format!("{}\n\nQuestion: {}\n\nAnswer: ", system_prompt, prompt),
        ];

        let mut best_output = String::new();

        // Try each prompt format
        for (i, prompt_format) in prompts_to_try.iter().enumerate() {
            // Start fresh with the context for each attempt
            if i > 0 {
                // Unfortunately, we can't really reset the session in this library version
                // Just proceed with the next prompt anyway
                // We'll skip all but the first prompt format if the first one works
            }

            // Feed prompt to the model
            self.session.advance_context(prompt_format)?;

            // Generate response
            let mut output = String::new();

            // Create a sampler with good settings for code generation
            let sampler = StandardSampler::new_softmax(
                vec![
                    llama_cpp::standard_sampler::SamplerStage::TopK(40),
                    llama_cpp::standard_sampler::SamplerStage::TopP(0.9),
                    llama_cpp::standard_sampler::SamplerStage::Temperature(0.7),
                ],
                1, // min_keep
            );

            let completions = self
                .session
                .start_completing_with(sampler, 2048)?
                .into_strings();

            for token in completions {
                output.push_str(&token);
            }

            // Clean up the output
            let clean_output = output
                .replace("<|im_end|>", "")
                .replace("<|assistant|>", "")
                .replace("<|user|>", "")
                .trim()
                .to_string();

            // If we got a reasonable response, use it
            if clean_output.len() > 50 {
                best_output = clean_output;
                break;
            }

            // If this is the last attempt, use whatever we got
            if i == prompts_to_try.len() - 1 && best_output.is_empty() {
                best_output = clean_output;
            }
        }

        Ok(best_output)
    }
}
