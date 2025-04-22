#!/bin/bash
set -e

# This is a simplified version to demonstrate the functionality

# Create a sample benchmark file
cat > /tmp/benchmark.md << 'EOF'
# Sample Benchmark File

## Results

<!-- BENCHMARK_RESULTS -->
_No benchmark data available yet._
<!-- END_BENCHMARK_RESULTS -->

## Footer
EOF

# Create sample content to insert
CONTENT="Here is the new benchmark data!
- Test 1: Pass
- Test 2: Pass
- Test 3: Fail"

# Update the file with the new content
{
  # Read up to the marker
  sed -n '1,/<!-- BENCHMARK_RESULTS -->/p' /tmp/benchmark.md

  # Insert the new content
  echo "$CONTENT"

  # Read from the end marker to the end of file
  sed -n '/<!-- END_BENCHMARK_RESULTS -->/,$p' /tmp/benchmark.md
} > /tmp/benchmark.md.tmp

# Move the updated file back
mv /tmp/benchmark.md.tmp /tmp/benchmark.md

# Show the result
echo "Updated benchmark file:"
cat /tmp/benchmark.md
