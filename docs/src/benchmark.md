# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-05-01 02:42:42 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 109880 ms |
| Tool Tests | 6/7 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (32468ms (32.46s))
- [x] test_glob_tool_with_llm (11679ms (11.67s))
- [x] test_grep_tool_with_llm (10843ms (10.84s))
- [x] test_ls_tool_with_llm (13374ms (13.37s))
- [x] test_edit_tool_with_llm (11317ms (11.31s))
- [x] test_bash_tool_with_llm (17572ms (17.57s))
- [ ] test_write_tool_with_llm (12584ms (12.58s))

<!-- END_BENCHMARK_RESULTS -->
