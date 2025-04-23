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
TOOL_RESULTS_FILE="$RESULTS_DIR/tool_tests/file_read_tool_results.json"
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

# Check if jq is installed (required for JSON parsing)
if ! command -v jq &> /dev/null; then
  echo "Error: jq is required for JSON parsing but not found"
  exit 1
fi

# Extract timestamp from summary
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S UTC")
if [ -f "$SUMMARY_FILE" ]; then
  TIMESTAMP_RAW=$(jq -r '.timestamp' "$SUMMARY_FILE" 2>/dev/null || echo "")
  if [ -n "$TIMESTAMP_RAW" ]; then
    # Keep ISO format for consistency, but format nicely if possible
    TIMESTAMP=$(date -d "$TIMESTAMP_RAW" +"%Y-%m-%d %H:%M:%S UTC" 2>/dev/null || echo "$TIMESTAMP_RAW")
  fi
fi

# Extract data from summary.json
MODEL="(unknown)"
TOOL_TIME="N/A"
TEST_TOTAL="0"
TEST_PASSED="0"

if [ -f "$SUMMARY_FILE" ]; then
  MODEL=$(jq -r '.model' "$SUMMARY_FILE" 2>/dev/null || echo "(unknown)")
  TOOL_TIME=$(jq -r '.tool_benchmark_ms' "$SUMMARY_FILE" 2>/dev/null || echo "N/A")
  TEST_TOTAL=$(jq -r '.test_summary.total // 0' "$SUMMARY_FILE" 2>/dev/null || echo "0")
  TEST_PASSED=$(jq -r '.test_summary.passed // 0' "$SUMMARY_FILE" 2>/dev/null || echo "0")
fi

# Format test results summary
TOOL_TEST_RESULTS="No test results available"
if [ -n "$TEST_TOTAL" ] && [ "$TEST_TOTAL" != "0" ]; then
  TOOL_TEST_RESULTS="$TEST_PASSED/$TEST_TOTAL tests passed"
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

# Create a variable for test details with checklist format
TEST_DETAILS=""

# Add test details if available
if [ -f "$TOOL_RESULTS_FILE" ]; then
  # Check if file contains valid JSON
  if jq -e . "$TOOL_RESULTS_FILE" > /dev/null 2>&1; then
    # Extract test details from the JSON file
    READS_FILES=$(jq -r '.test_details.capabilities.reads_files // false' "$TOOL_RESULTS_FILE")
    EXTRACTS_LINES=$(jq -r '.test_details.capabilities.extracts_specific_lines // false' "$TOOL_RESULTS_FILE")
    TEST_TIME=$(jq -r '.test_details.test_time_seconds // ""' "$TOOL_RESULTS_FILE")
    
    # Build bullet checklist (without checkmarks)
    TEST_DETAILS="${TEST_DETAILS}- [ ] Agent correctly reads files and processes file content
"
    TEST_DETAILS="${TEST_DETAILS}- [ ] Agent can identify and extract specific lines from files
"

    # If tests were successful, mark the corresponding list items
    if [ "$READS_FILES" = "true" ]; then
      TEST_DETAILS=$(echo "$TEST_DETAILS" | sed 's/- \[ \] Agent correctly reads files/- [x] Agent correctly reads files/')
    fi
    
    if [ "$EXTRACTS_LINES" = "true" ]; then
      TEST_DETAILS=$(echo "$TEST_DETAILS" | sed 's/- \[ \] Agent can identify and extract/- [x] Agent can identify and extract/')
    fi

    # Add additional items from the benchmark file's Tool Performance Summary
    TEST_DETAILS="${TEST_DETAILS}- [ ] Agent has high tool selection accuracy
"
    TEST_DETAILS="${TEST_DETAILS}- [ ] Real agent successfully reads files with Ollama LLM
"

    # Add execution time info if available
    if [ -n "$TEST_TIME" ]; then
      TEST_DETAILS="${TEST_DETAILS}ℹ️ Test execution time: ${TEST_TIME}s
"
    fi
  else
    # Fallback if JSON is invalid
    TEST_DETAILS="${TEST_DETAILS}- [ ] File read tool tests were executed but results format is invalid
"
  fi
else
  TEST_DETAILS="_Detailed test results not available_
"
fi

# Update the benchmark markdown file
if [ -f "$BENCHMARK_FILE" ]; then
  # Use the sed approach that works across different environments
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