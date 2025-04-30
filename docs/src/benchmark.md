# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-30 18:30:55 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:7b` |
| Tool Benchmark Time | 122100 ms |
| Tool Tests | 4/6 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (39347ms (39.34s))
- [x] test_glob_tool_with_llm (6662ms (6.66s))
- [x] test_grep_tool_with_llm (12435ms (12.43s))
- [x] test_ls_tool_with_llm (20513ms (20.51s))
- [ ] test_edit_tool_with_llm (22781ms (22.78s))
- [ ] test_replace_tool_with_llm (20321ms (20.32s))

<!-- END_BENCHMARK_RESULTS -->
