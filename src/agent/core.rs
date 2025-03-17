use crate::agent::executor::AgentExecutor;
use crate::apis::anthropic::AnthropicClient;
use crate::apis::api_client::{ApiClientEnum, DynApiClient};
use crate::apis::openai::OpenAIClient;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum LLMProvider {
    Anthropic,
    OpenAI,
}

#[derive(Clone)]
pub struct Agent {
    provider: LLMProvider,
    model: Option<String>,
    api_client: Option<DynApiClient>,
    system_prompt: Option<String>,
    progress_sender: Option<mpsc::Sender<String>>,
}

impl Agent {
    pub fn new(provider: LLMProvider) -> Self {
        Self {
            provider,
            model: None,
            api_client: None,
            system_prompt: None,
            progress_sender: None,
        }
    }

    pub fn new_with_api_key(provider: LLMProvider, api_key: String) -> Self {
        // Create a new agent with the given provider and API key
        // The API key will be used during initialization
        let mut agent = Self::new(provider);
        // Store the API key as the model temporarily
        // It will be handled properly in initialize_with_api_key
        agent.model = Some(api_key);
        agent
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
    }

    pub fn with_progress_sender(mut self, sender: mpsc::Sender<String>) -> Self {
        self.progress_sender = Some(sender);
        self
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Create the API client based on provider and model
        self.api_client = Some(match self.provider {
            LLMProvider::Anthropic => {
                let client = AnthropicClient::new(self.model.clone())?;
                ApiClientEnum::Anthropic(Arc::new(client))
            }
            LLMProvider::OpenAI => {
                let client = OpenAIClient::new(self.model.clone())?;
                ApiClientEnum::OpenAi(Arc::new(client))
            }
        });

        Ok(())
    }

    pub async fn initialize_with_api_key(&mut self, api_key: String) -> Result<()> {
        // Create the API client based on provider and model, using the provided API key
        self.api_client = Some(match self.provider {
            LLMProvider::Anthropic => {
                let client = AnthropicClient::with_api_key(api_key, self.model.clone())?;
                ApiClientEnum::Anthropic(Arc::new(client))
            }
            LLMProvider::OpenAI => {
                // For now we don't support custom API keys for OpenAI, fallback to env var
                let client = OpenAIClient::new(self.model.clone())?;
                ApiClientEnum::OpenAi(Arc::new(client))
            }
        });

        Ok(())
    }

    pub async fn execute(&self, query: &str) -> Result<String> {
        let api_client = self
            .api_client
            .as_ref()
            .context("Agent not initialized. Call initialize() first.")?;

        // Create and configure executor
        let mut executor = AgentExecutor::new(api_client.clone());

        // Add progress sender if available
        if let Some(sender) = &self.progress_sender {
            executor = executor.with_progress_sender(sender.clone());
        }

        // Add system prompt if available
        if let Some(system_prompt) = &self.system_prompt {
            executor.add_system_message(system_prompt.clone());
        } else {
            // Use default system prompt
            executor.add_system_message(DEFAULT_SYSTEM_PROMPT.to_string());
        }

        // Add user query
        executor.add_user_message(query.to_string());

        // Execute and return result
        executor.execute().await
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are OLI Code Assistant, a powerful coding assistant designed to help with software development tasks.

## YOUR ROLE
You are a highly specialized coding assistant built to help developers with programming tasks, code understanding, debugging, and software development.

## CAPABILITIES
1. Reading and understanding code files
2. Searching code repositories efficiently
3. Editing and creating code files with precision
4. Running shell commands and interpreting results
5. Answering technical coding questions
6. Debugging and solving programming issues
7. Working with multiple programming languages and frameworks

## HANDLING USER QUERIES
When a user asks a question:
1. FIRST, determine if the question is about code, programming, or software development:
   - If YES: Use your tools to explore the code, understand context, and provide a helpful response
   - If NO: Politely explain that you're specialized for programming tasks and suggest how you can help with software development

2. For relevant technical questions, ALWAYS use tools to explore the codebase before answering:
   - For questions about files or code structure, use LS or GlobTool to explore
   - For questions about code functionality, use View to read files and understand the code
   - For questions about specific implementations, use GrepTool to find relevant code patterns

3. NEVER invent or assume code exists without checking - use tools to verify

## WORKFLOW GUIDELINES
When helping users:
- Always use tools to explore code and understand context before answering
- Break down complex tasks into manageable steps
- Be thorough while remaining concise in your responses
- Focus on practical, working solutions that follow best practices
- When working with code, ensure proper error handling and edge cases
- Verify your solutions when possible

## AVAILABLE TOOLS
You have access to the following tools that you should use proactively:

- View: Read files from the filesystem
  Usage: Use this to examine file contents when you need to understand existing code

- GlobTool: Find files matching patterns like "**/*.rs"
  Usage: Use this to locate files by name patterns when searching through a repository

- GrepTool: Search file contents using regular expressions
  Usage: Use this to find specific code patterns or text within files

- LS: List directory contents
  Usage: Use this to explore project structure and available files/directories

- Edit: Make targeted edits to files
  Usage: Use this for precise modifications to existing files

- Replace: Completely replace or create files
  Usage: Use this when creating new files or completely rewriting existing ones

- Bash: Execute shell commands
  Usage: Use this to run commands, execute tests, or perform system operations

## COMMUNICATION APPROACH
- Be direct and to the point
- Use precise technical language
- Format code with proper syntax highlighting
- When explaining complex concepts, use examples
- Admit when you're unsure rather than guessing
- Be solution-oriented and practical

## OUTPUT QUALITY
Always ensure your code and suggestions are:
- Syntactically correct
- Following language idioms and best practices
- Properly indented and formatted
- Well-commented when appropriate
- Optimized for readability and maintainability
- Tested or verifiable when possible

Always prioritize being helpful, accurate, and providing working solutions that follow modern software development practices.
"#;
