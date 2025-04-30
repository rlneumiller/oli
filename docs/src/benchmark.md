# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-30 19:49:04 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 82078 ms |
| Tool Tests | 6/6 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (39554ms (39.55s))
- [x] test_glob_tool_with_llm (8820ms (8.82s))
- [x] test_grep_tool_with_llm (9391ms (9.39s))
- [x] test_ls_tool_with_llm (4704ms (4.70s))
- [x] test_edit_tool_with_llm (12423ms (12.42s))
- [x] test_replace_tool_with_llm (7150ms (7.15s))

<!-- END_BENCHMARK_RESULTS -->
