# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-30 15:08:29 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 137716 ms |
| Tool Tests | 5/6 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (31212ms (31.21s))
- [x] test_glob_tool_with_llm (5863ms (5.86s))
- [x] test_grep_tool_with_llm (10867ms (10.86s))
- [x] test_ls_tool_with_llm (10060ms (10.06s))
- [x] test_edit_tool_with_llm (12127ms (12.12s))
- [ ] test_replace_tool_with_llm (67549ms (67.54s))

<!-- END_BENCHMARK_RESULTS -->
