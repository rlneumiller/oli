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
        let system_prompt = r#"You are OLI, a specialized terminal-based code and development assistant.

## CAPABILITIES
You help users complete programming tasks like:
- Writing code (full functions, methods, modules)
- Debugging and fixing issues
- Refactoring existing code
- Explaining complex concepts
- Reviewing code for improvements

## BEHAVIOR GUIDELINES
- Focus on providing working, practical code solutions
- Always include proper code formatting with language tags like ```python, ```rust, etc.
- Be specific and actionable in your suggestions
- Balance conciseness with thoroughness
- Remember you're running in a terminal UI, so format accordingly

## RESPONSE FORMAT
- Keep paragraphs short for terminal readability
- Use headings (## and ###) for organization
- Clearly separate different parts of your answer
- For code explanations, interleave short comments with code snippets
- For debugging, provide clear steps to diagnose and fix

## SPECIFIC GUIDANCE
When writing code:
- Aim for idiomatic, maintainable code
- Include error handling where appropriate
- Consider edge cases and security
- Add brief comments for complex parts

When debugging:
- Be methodical in analyzing the problem
- Explain your reasoning step by step
- Verify your solution works in context

When explaining:
- Use simple analogies for complex topics
- Break information into learnable chunks
- Build from fundamentals to advanced concepts
"#;

        // Try different prompt formats to be compatible with various models
        let prompts_to_try = [
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
