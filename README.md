[![Crates.io](https://img.shields.io/crates/v/oli-tui?style=flat-square)](https://crates.io/crates/oli-tui)
[![Docs.rs](https://img.shields.io/badge/docs.rs-latest-blue?style=flat-square)](https://docs.rs/oli-tui)
[![License](https://img.shields.io/badge/license-Apache_2.0-blue?style=flat-square)](https://opensource.org/license/apache-2-0)
[![Codecov](https://img.shields.io/codecov/c/github/amrit110/oli?style=flat-square)](https://codecov.io/github/amrit110/oli)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg?logo=rust&style=flat-square)](https://www.rust-lang.org)

# OLI - Open Local Intelligent assistant

OLI is an open-source alternative to Claude Code, built in Rust to provide powerful agentic capabilities for coding assistance. It features:

- A flexible TUI interface for working with code
- Support for both local LLMs (via vLLM or ollama) and cloud APIs (currently only tested using Anthropic Claude Sonnet 3.7 and OpenAI GPT4o but other APIs and local LLM support coming soon!)
- Strong agentic capabilities including file search, edit, and command execution

⚠️ This project is in a very early stage and is prone to bugs and issues! Please post your issues as you encounter them.

## Installation

### Using Cargo

```bash
cargo install oli-tui
```

### Using Homebrew (macOS)

```bash
brew tap amrit110/oli
brew install oli
```

### From Source

```bash
# Clone the repository
git clone https://github.com/amrit110/oli
cd oli

# Build and run
cargo build --release
cargo run
```

## Environment Setup

For API-based features, set up your environment variables:

```bash
# Create a .env file in the project root
echo "ANTHROPIC_API_KEY=your_key_here" > .env
# OR
echo "OPENAI_API_KEY=your_key_here" > .env
```

### Using Anthropic Claude 3.7 Sonnet (Recommended)

Claude 3.7 Sonnet provides the most reliable and advanced agent capabilities:

1. Obtain an API key from [Anthropic](https://www.anthropic.com/)
2. Set the ANTHROPIC_API_KEY environment variable
3. Select the "Claude 3.7 Sonnet" model in the UI

This implementation includes:
- Optimized system prompts for Claude 3.7
- JSON schema output formatting for structured responses
- Improved error handling and retry mechanisms

## Usage

1. Start the application:
```bash
cargo run
```

2. Select a model:
   - Cloud models (Claude 3 Sonnet, GPT-4o) for full agent capabilities
   - Local models via vllm or ollama (coming soon!)

3. Make your coding query in the chat interface:
   - Ask for file searches
   - Request code edits
   - Execute shell commands
   - Get explanations of code

## Examples

Here are some example queries to try:

- "Explain the codebase and how to get started"
- "List all files in the project"
- "Summarize the Cargo.toml file"
- "Show me all files that import the 'anyhow' crate"

## License

This project is licensed under the Apache 2.0 License - see the LICENSE file for details.

## Acknowledgments

- This project is inspired by Claude Code and similar AI assistants
- Uses Anthropic's Claude 3.7 Sonnet model for optimal agent capabilities
- Built with Rust and the Ratatui library for terminal UI
- Special thanks to the Rust community for excellent libraries and tools
