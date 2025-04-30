# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-30 14:21:18 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 108566 ms |
| Tool Tests | 5/5 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (40178ms (40.17s))
- [x] test_glob_tool_with_llm (13533ms (13.53s))
- [x] test_grep_tool_with_llm (24222ms (24.22s))
- [x] test_ls_tool_with_llm (11519ms (11.51s))
- [x] test_replace_tool_with_llm (19085ms (19.08s))

<!-- END_BENCHMARK_RESULTS -->
