# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-27 01:47:30 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:14b` |
| Tool Benchmark Time | 148592 ms |
| Tool Tests | 4/4 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (72007ms (72.00s))
- [x] test_glob_tool_with_llm (28000ms (28.00s))
- [x] test_grep_tool_with_llm (26904ms (26.90s))
- [x] test_ls_tool_with_llm (21654ms (21.65s))

<!-- END_BENCHMARK_RESULTS -->
