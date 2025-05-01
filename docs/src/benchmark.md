# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-05-01 15:06:00 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 109649 ms |
| Tool Tests | 6/7 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (35658ms (35.65s))
- [x] test_glob_tool_with_llm (13154ms (13.15s))
- [x] test_grep_tool_with_llm (12659ms (12.65s))
- [x] test_ls_tool_with_llm (11260ms (11.26s))
- [x] test_edit_tool_with_llm (11366ms (11.36s))
- [x] test_bash_tool_with_llm (11725ms (11.72s))
- [ ] test_write_tool_with_llm (13786ms (13.78s))

<!-- END_BENCHMARK_RESULTS -->
