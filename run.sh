#!/bin/bash

# Run script for oli (Rust backend + React/Ink frontend)
echo "Starting oli (Hybrid Rust/React Application)"

# Get the script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
BACKEND_PATH="$SCRIPT_DIR/target/release/oli-server"

# Copy the server to current directory for the UI to find
cp "$BACKEND_PATH" "$SCRIPT_DIR/oli-server" 2>/dev/null || true

# Start UI directly (it will spawn backend)
if [ -d "$SCRIPT_DIR/ui" ]; then
  echo "Starting UI client..."
  cd "$SCRIPT_DIR/ui" || exit 1
  npm run build && npm run start
  exit $?
else
  echo "Error: UI directory not found"
  exit 1
fi
