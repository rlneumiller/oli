#!/bin/bash
set -e

# Run script for Oli (Rust backend + React/Ink frontend)
echo "Starting Oli (Hybrid Rust/React Application)"

# Check if the backend build exists
if [ ! -f "target/release/oli" ] && [ ! -f "../target/release/oli" ]; then
  echo "Error: Backend not built. Please run ./build.sh first."
  exit 1
fi

# Get the current directory name
CURRENT_DIR=$(basename "$PWD")

# Check if frontend build exists based on current directory
if [ "$CURRENT_DIR" = "ui" ]; then
  if [ ! -d "dist" ]; then
    echo "Error: Frontend not built. Please run ./build.sh first."
    exit 1
  fi
else
  if [ ! -d "ui/dist" ]; then
    echo "Error: Frontend not built. Please run ./build.sh first."
    exit 1
  fi
fi

# Run the application
# Get the current directory name
CURRENT_DIR=$(basename "$PWD")

# Check if we're already in the UI directory or need to navigate to it
if [ "$CURRENT_DIR" = "ui" ]; then
  # Already in the UI directory
  node --import tsx dist/cli.js "$@"
else
  # Navigate to the UI directory
  cd ui || exit 1
  node --import tsx dist/cli.js "$@"
fi
