#!/bin/bash
set -e

# For portability across different bash versions
export LC_ALL=C

# This script updates the benchmark.md documentation file with the latest benchmark results
# It extracts data from the benchmark_results directory and formats it as a Markdown table

# Usage: ./update_benchmark_docs.sh

# Check if we're running in a GitHub Actions environment
if [ -n "$GITHUB_WORKSPACE" ]; then
  cd "$GITHUB_WORKSPACE"
fi

# Paths
RESULTS_DIR="benchmark_results"
DOCS_DIR="docs/src"
BENCHMARK_FILE="$DOCS_DIR/benchmark.md"
SUMMARY_FILE="$RESULTS_DIR/summary.json"
TOOL_RESULTS_FILE="$RESULTS_DIR/tool_tests/file_read_tool_results.txt"
TOOL_SUMMARY_FILE="$RESULTS_DIR/tool_tests/summary.json"

# Check if results exist
if [ ! -d "$RESULTS_DIR" ]; then
  echo "Error: Benchmark results directory not found"
  exit 1
fi

if [ ! -f "$BENCHMARK_FILE" ]; then
  echo "Error: Benchmark markdown file not found"
  exit 1
fi

# Extract timestamp from summary if it exists
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S UTC")
if [ -f "$SUMMARY_FILE" ]; then
  TIMESTAMP_RAW=$(jq -r '.timestamp' "$SUMMARY_FILE" 2>/dev/null || echo "")
  if [ -n "$TIMESTAMP_RAW" ]; then
    # Convert ISO format to more readable format
    TIMESTAMP=$(date -d "$TIMESTAMP_RAW" +"%Y-%m-%d %H:%M:%S UTC" 2>/dev/null || echo "$TIMESTAMP_RAW")
  fi
fi

# Extract model name
MODEL="(unknown)"
if [ -f "$SUMMARY_FILE" ]; then
  MODEL=$(jq -r '.model' "$SUMMARY_FILE" 2>/dev/null || echo "(unknown)")
fi

# Extract tool benchmark time
TOOL_TIME="N/A"
if [ -f "$SUMMARY_FILE" ]; then
  TOOL_TIME=$(jq -r '.tool_benchmark_ms' "$SUMMARY_FILE" 2>/dev/null || echo "N/A")
fi

# Check if the tool test results file exists
TOOL_TEST_RESULTS="No test results available"
if [ -f "$TOOL_RESULTS_FILE" ]; then
  # Count integration tests (our real benchmark tests)
  TOTAL_TESTS=$(grep -c "test integration::test_file_read_tool::" "$TOOL_RESULTS_FILE" 2>/dev/null || echo "0")

  # Count successful tests (look for "... ok" lines related to our specific tests)
  SUCCESS_COUNT=$(grep "test integration::test_file_read_tool::" "$TOOL_RESULTS_FILE" | grep -c "ok" 2>/dev/null || echo "0")

  # Simplified test results display
  TOOL_TEST_RESULTS="$SUCCESS_COUNT/$TOTAL_TESTS tests passed"
fi

# Create the markdown table content
MARKDOWN_CONTENT="## Latest Results (as of $TIMESTAMP)

| Category | Details |
|----------|---------|
| Model | \`$MODEL\` |
| Tool Benchmark Time | $TOOL_TIME ms |
| Tool Tests | $TOOL_TEST_RESULTS |

### Tool Performance Summary
"

# Create a variable for test details
TEST_DETAILS=""

# Add test details if available
if [ -f "$TOOL_RESULTS_FILE" ]; then
  # Extract test details (specific to the file_read_tool tests)
  # Look for "test_agent_uses_file_read_tool" and extract result
  if grep -q "test_agent_uses_file_read_tool" "$TOOL_RESULTS_FILE"; then
    if grep -q "test_agent_uses_file_read_tool.*ok" "$TOOL_RESULTS_FILE"; then
      TEST_DETAILS="${TEST_DETAILS}✅ Agent correctly uses FileReadTool for direct path prompts
"
    else
      TEST_DETAILS="${TEST_DETAILS}❌ Agent failed to use FileReadTool for direct path prompts
"
    fi
  fi

  # Look for "test_file_read_tool_with_offset_limit" and extract result
  if grep -q "test_file_read_tool_with_offset_limit" "$TOOL_RESULTS_FILE"; then
    if grep -q "test_file_read_tool_with_offset_limit.*ok" "$TOOL_RESULTS_FILE"; then
      TEST_DETAILS="${TEST_DETAILS}✅ FileReadTool correctly handles offset and limit parameters
"
    else
      TEST_DETAILS="${TEST_DETAILS}❌ FileReadTool failed to handle offset and limit parameters
"
    fi
  fi

  # Look for "test_agent_tool_selection_accuracy" and extract result
  if grep -q "test_agent_tool_selection_accuracy" "$TOOL_RESULTS_FILE"; then
    if grep -q "test_agent_tool_selection_accuracy.*ok" "$TOOL_RESULTS_FILE"; then
      TEST_DETAILS="${TEST_DETAILS}✅ Agent has high tool selection accuracy
"
    else
      TEST_DETAILS="${TEST_DETAILS}❌ Agent failed tool selection accuracy tests
"
    fi
  fi

  # Look for "test_real_agent_file_read" and extract result
  if grep -q "test_real_agent_file_read" "$TOOL_RESULTS_FILE"; then
    if grep -q "test_real_agent_file_read.*ok" "$TOOL_RESULTS_FILE"; then
      TEST_DETAILS="${TEST_DETAILS}✅ Real agent successfully reads files with Ollama LLM
"
    else
      TEST_DETAILS="${TEST_DETAILS}❌ Real agent failed to read files with Ollama LLM
"
    fi
  fi
else
  TEST_DETAILS="_Detailed test results not available_
"
fi

# Update the benchmark markdown file
if [ -f "$BENCHMARK_FILE" ]; then
  # Use the simplified approach that works
  {
    # Read up to the marker
    sed -n '1,/<!-- BENCHMARK_RESULTS -->/p' "$BENCHMARK_FILE"

    # Insert the new content with test details
    echo "$MARKDOWN_CONTENT$TEST_DETAILS"

    # Read from the end marker to the end of file
    sed -n '/<!-- END_BENCHMARK_RESULTS -->/,$p' "$BENCHMARK_FILE"
  } > "${BENCHMARK_FILE}.tmp"

  # Move the updated file back
  mv "${BENCHMARK_FILE}.tmp" "$BENCHMARK_FILE"

  echo "Updated benchmark documentation at $BENCHMARK_FILE"
else
  echo "Warning: Benchmark markdown file not found at $BENCHMARK_FILE"
fi

# If running in GitHub Actions, configure git for committing
if [ -n "$GITHUB_ACTOR" ]; then
  git config --global user.name "GitHub Actions"
  git config --global user.email "actions@github.com"

  # Check if there are changes to commit
  if git diff --quiet "$BENCHMARK_FILE"; then
    echo "No changes to commit"
  else
    echo "Committing updated benchmark documentation"
    git add "$BENCHMARK_FILE"
    git commit -m "Update benchmark results [skip ci]"
  fi
fi
