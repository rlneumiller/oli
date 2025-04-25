# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-25 16:10:23 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:14b` |
| Tool Benchmark Time | 122930 ms |
| Tool Tests | 4/4 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (58769ms (58.76s))
- [x] test_glob_tool_with_llm (21775ms (21.77s))
- [x] test_grep_tool_with_llm (20327ms (20.32s))
- [x] test_ls_tool_with_llm (22031ms (22.03s))

<!-- END_BENCHMARK_RESULTS -->
