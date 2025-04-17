#!/bin/bash
set -e

# Build script for oli (Rust backend + React/Ink frontend)
echo "Building oli (Hybrid Rust/React Application)"

# Check for required tools
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo is not installed. Please install Rust."; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "Error: npm is not installed. Please install Node.js."; exit 1; }

# Build the Rust backend
echo "=== Building Rust backend ==="
cargo build --release

# Build the React/Ink frontend
echo "=== Building React frontend ==="

# Get the current directory name
CURRENT_DIR=$(basename "$PWD")

# Check if we're already in the UI directory or need to navigate to it
if [ "$CURRENT_DIR" = "ui" ]; then
  # Already in the UI directory
  npm install
  npm run build
elif [ -d "ui" ]; then
  # Navigate to the UI directory
  cd ui || exit 1
  npm install
  npm run build
  cd ..
else
  echo "Error: UI directory not found"
  exit 1
fi

# Create the single executable that combines both
echo "=== Creating combined cli package ==="

# Create the package directory
mkdir -p dist/oli

# Copy the binaries and UI files
cp target/release/oli-server dist/oli/
cp target/release/oli dist/oli/oli-bin
mkdir -p dist/oli/ui
cp -r ui/dist/* dist/oli/ui/ 2>/dev/null || echo "Warning: UI dist files not found"
# Copy node_modules for dependencies
mkdir -p dist/oli/ui/node_modules
cp -r ui/node_modules dist/oli/ui/ 2>/dev/null || echo "Warning: UI node_modules not found"

# Create the wrapper script
cat > dist/oli/oli << 'EOF'
#!/bin/bash

# Find the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Start the server in the background
if [ -f "$SCRIPT_DIR/oli-server" ]; then
  "$SCRIPT_DIR/oli-server" &
  SERVER_PID=$!

  # Start the UI
  cd "$SCRIPT_DIR/ui"
  NODE_PATH="$SCRIPT_DIR/ui/node_modules" node cli.js "$@"

  # Kill the server when the UI exits
  kill $SERVER_PID 2>/dev/null
else
  # Fallback to the binary version if server isn't available
  "$SCRIPT_DIR/oli-bin" "$@"
fi
EOF

chmod +x dist/oli/oli

echo "=== Build complete ==="
echo "Binary is available at: ./dist/oli/oli"
echo "To run the application, use: './dist/oli/oli'"
