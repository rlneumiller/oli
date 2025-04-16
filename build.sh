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

echo "=== Build complete ==="
echo "To run the application, use: './run.sh'"
