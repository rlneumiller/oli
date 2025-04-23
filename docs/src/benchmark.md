# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's FileReadTool operates when used with local Ollama models. The test evaluates:

1. File reading capability - Can the agent correctly read a file when given a path?
2. Content processing - Can the agent identify and extract specific content (like a particular line)?
3. Execution time - How quickly can the agent complete a file reading task?

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-22T19:01:36Z)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 23000 ms |
| Tool Tests | 4/4 tests passed |

### Tool Performance Summary
- [x] Agent correctly reads files and processes file content
- [x] Agent can identify and extract specific lines from files
- [x] Agent has high tool selection accuracy
- [x] Real agent successfully reads files with Ollama LLM

ℹ️ Test execution time: 3.5s

<!-- END_BENCHMARK_RESULTS -->

## Methodology

The benchmark uses Ollama's local models to test real-world file reading capability. The test:

1. Creates a temporary file with known line-by-line content
2. Initializes an agent with the specified Ollama model
3. Prompts the agent to read the file and identify specific line content
4. Validates that the agent can correctly understand and process the file contents
5. Measures execution time and success rate

The test is designed to be lightweight yet comprehensive, focusing on essential functionality while maintaining consistent performance metrics across different models. All tests are executed in a controlled CI environment with consistent model versions.
