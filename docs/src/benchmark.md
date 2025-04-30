# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-30 17:01:14 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 123256 ms |
| Tool Tests | 4/6 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (38151ms (38.15s))
- [ ] test_glob_tool_with_llm (5375ms (5.37s))
- [x] test_grep_tool_with_llm (19075ms (19.07s))
- [x] test_ls_tool_with_llm (11632ms (11.63s))
- [ ] test_edit_tool_with_llm (38889ms (38.88s))
- [x] test_replace_tool_with_llm (10097ms (10.09s))

<!-- END_BENCHMARK_RESULTS -->
