# oli tool benchmarks

This page contains the latest benchmark results for oli tool use performance.
These benchmarks are automatically updated with each new PR.

## Tool Performance Overview

The benchmark test measures how efficiently oli's tools operate when used with local
Ollama models. The benchmark evaluates every tool's performance using simple test cases.

## Latest Benchmark Results

_This section is automatically updated by CI/CD pipelines._

<!-- BENCHMARK_RESULTS -->
## Latest Results (as of 2025-04-27 05:12:50 UTC)

| Category | Details |
|----------|---------|
| Model | `qwen2.5-coder:14b` |
| Tool Benchmark Time | 115935 ms |
| Tool Tests | 4/4 tests passed |

### Tool Performance Tests
- [x] test_read_file_tool_with_llm (43771ms (43.77s))
- [x] test_glob_tool_with_llm (27521ms (27.52s))
- [x] test_grep_tool_with_llm (22549ms (22.54s))
- [x] test_ls_tool_with_llm (22065ms (22.06s))

<!-- END_BENCHMARK_RESULTS -->
