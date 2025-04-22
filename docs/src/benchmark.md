# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark tests measure how efficiently oli tools operate when used with local Ollama models. The tests evaluate:

1. Tool selection accuracy - Does the agent correctly choose the FileReadTool when appropriate?
2. Functionality with various parameters - Does the tool handle offset and limit parameters correctly?
3. Execution time - How quickly can the tool process file operations?

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
✅ Agent correctly uses FileReadTool for direct path prompts
✅ FileReadTool correctly handles offset and limit parameters
✅ Agent has high tool selection accuracy
✅ Real agent successfully reads files with Ollama LLM

<!-- END_BENCHMARK_RESULTS -->

## Methodology

The benchmarks use Ollama's local models to test real-world tool usage. Each test:

1. Creates temporary files with known content
2. Prompts the agent to perform file reading operations
3. Verifies tool selection accuracy and content retrieval
4. Records execution time and success rates

All tests are executed in a controlled CI environment with consistent model versions.
