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
TOOL_RESULTS_FILE="$RESULTS_DIR/tool_tests/tools_benchmark_results.json"
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

### Tool Performance Tests
"

# Create a variable for test details with checklist format
TEST_DETAILS=""

# Dynamically find all benchmark tests in the codebase
# This will automatically pick up new benchmark tests when they're added
find_benchmark_tests() {
  local tests_dir="$1"
  if [ ! -d "$tests_dir" ]; then
    echo "Warning: Tests directory not found at $tests_dir" >&2
    return
  fi

  # Find all Rust files containing benchmark feature attributes
  local benchmark_files=$(find "$tests_dir" -type f -name "*.rs" -exec grep -l "cfg_attr.*feature.*benchmark.*ignore" {} \;)

  # Initialize empty array for tests
  local found_tests=()

  # Process each file
  for file in $benchmark_files; do
    # Extract test function names using grep and sed
    # Look for the pattern: async fn test_name that appears after a line with the benchmark attribute
    local tests=$(grep -A 1 "cfg_attr.*feature.*benchmark.*ignore" "$file" |
                 grep -o "async fn test[a-zA-Z0-9_]*" |
                 sed 's/async fn //g')

    # Add each test to the array
    for test in $tests; do
      found_tests+=("$test")
    done
  done

  # Return the found tests
  echo "${found_tests[@]}"
}

# Get all benchmark tests
if [ -n "$GITHUB_WORKSPACE" ]; then
  # If running in GitHub Actions
  TESTS_DIR="$GITHUB_WORKSPACE/tests"
else
  # If running locally
  TESTS_DIR="./tests"
fi

# Find benchmark tests and add them to ALL_TESTS array
IFS=" " read -r -a ALL_TESTS <<< "$(find_benchmark_tests "$TESTS_DIR")"

# Print discovered tests for logging
echo "Discovered benchmark tests: ${ALL_TESTS[*]}" >&2

# Add test execution time if available
TEST_TIME=""
if [ -f "$TOOL_RESULTS_FILE" ] && jq -e . "$TOOL_RESULTS_FILE" > /dev/null 2>&1; then
  TEST_TIME=$(jq -r '.test_details.test_time_seconds // ""' "$TOOL_RESULTS_FILE")
fi

# Get the list of passed tests and individual test times
PASSED_TESTS=()
declare -A TEST_TIMES

# If the tool results file exists and contains raw output, extract data
if [ -f "$TOOL_RESULTS_FILE" ] && jq -e . "$TOOL_RESULTS_FILE" > /dev/null 2>&1; then
  # Extract the raw output from the test results file
  RAW_OUTPUT=$(jq -r '.raw_output // ""' "$TOOL_RESULTS_FILE")

  # Filter to only keep benchmark tests (those with "_with_llm" in their name)
  BENCHMARK_TESTS=()
  for test in "${ALL_TESTS[@]}"; do
    if [[ "$test" == *"_with_llm"* ]]; then
      BENCHMARK_TESTS+=("$test")
    fi
  done

  # Check each benchmark test individually by looking for "test::...::test_name ... ok" pattern in raw output
  for test in "${BENCHMARK_TESTS[@]}"; do
    # Check if the test passed
    if echo "$RAW_OUTPUT" | grep -q "$test.*ok"; then
      PASSED_TESTS+=("$test")
    fi

    # Extract the test's individual timing information
    # First try to get it from the capabilities JSON section which should have accurate times
    if jq -e ".test_details.capabilities.${test#test_}.time" "$TOOL_RESULTS_FILE" > /dev/null 2>&1; then
      TIME_FOUND=$(jq -r ".test_details.capabilities.${test#test_}.time" "$TOOL_RESULTS_FILE")
      if [ -n "$TIME_FOUND" ] && [ "$TIME_FOUND" != "null" ]; then
        # Check if the time already has parentheses, add them if not
        if [[ "$TIME_FOUND" != \(* ]]; then
          TEST_TIMES["$test"]="($TIME_FOUND)"
        else
          TEST_TIMES["$test"]="$TIME_FOUND"
        fi
      fi
    else
      # Fall back to traditional pattern matching if JSON extraction fails
      # Look for a pattern like: test agent::test_tools::test_glob_tool_with_llm ... ok (12.34s)
      TEST_TIME_PATTERN="$test[^(]*(\([0-9.]+s\))"
      if TIME_FOUND=$(echo "$RAW_OUTPUT" | grep -o "$TEST_TIME_PATTERN" | grep -o "([0-9.]\+s)"); then
        # Store the time including parentheses
        TEST_TIMES["$test"]="$TIME_FOUND"
      fi
    fi
  done

  # If we couldn't find individual test times, extract them from the raw output differently
  if [ ${#TEST_TIMES[@]} -eq 0 ]; then
    echo "Attempting to extract individual test times from raw output..."
    # Look for timing patterns in the raw output for each benchmark test
    for test in "${BENCHMARK_TESTS[@]}"; do
      # First try to find our custom timing entries
      if TIME_FOUND=$(echo "$RAW_OUTPUT" | grep "Individual test time for $test" | grep -o "[0-9]\+ms ([0-9.]\+s)" | head -1); then
        TEST_TIMES["$test"]="($TIME_FOUND)"
      # Then try the usual pattern
      elif TIME_FOUND=$(echo "$RAW_OUTPUT" | grep "$test" | grep -o "finished in [0-9.]\+s" | head -1 | grep -o "[0-9.]\+s"); then
        TEST_TIMES["$test"]="($TIME_FOUND)"
      fi
    done
  fi
fi

# Update TEST_TOTAL and TEST_PASSED to reflect only benchmark tests
TEST_TOTAL=${#BENCHMARK_TESTS[@]}
TEST_PASSED=${#PASSED_TESTS[@]}

# Fall back to assuming all tests passed if the summary says so
if [ "$TEST_PASSED" = "$TEST_TOTAL" ] && [ "$TEST_TOTAL" -gt 0 ] && [ ${#PASSED_TESTS[@]} -eq 0 ]; then
  PASSED_TESTS=("${BENCHMARK_TESTS[@]}")
fi

# Generate the checklist with individual test times - only for benchmark tests
for test in "${BENCHMARK_TESTS[@]}"; do
  test_display="${test}"

  # Add individual execution time in brackets if available for this test
  if [ -n "${TEST_TIMES[$test]}" ]; then
    test_display="${test} ${TEST_TIMES[$test]}"
  # Fall back to overall time only if individual time not available
  elif [ -n "$TEST_TIME" ]; then
    test_display="${test} (${TEST_TIME}s)"
  fi

  if [[ " ${PASSED_TESTS[*]} " =~ " ${test} " ]]; then
    TEST_DETAILS="${TEST_DETAILS}- [x] ${test_display}
"
  else
    TEST_DETAILS="${TEST_DETAILS}- [ ] ${test_display}
"
  fi
done

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
