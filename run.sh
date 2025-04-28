#!/bin/bash
set -e

# Color codes
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Run script for oli (Rust backend + React/Ink frontend)
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Starting oli (Hybrid Rust/React Application)${NC}"
echo -e "${BLUE}======================================${NC}"

# Get the script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
BACKEND_PATH="$SCRIPT_DIR/target/release/oli-server"

echo -e "${YELLOW}Current directory:${NC} $(pwd)"
echo -e "${YELLOW}Script directory:${NC} $SCRIPT_DIR"
echo -e "${YELLOW}Backend path:${NC} $BACKEND_PATH"

# Check if App directory exists
if [ ! -d "$SCRIPT_DIR/app" ]; then
  echo -e "${RED}Error: App directory not found${NC}"
  exit 1
fi

# Install UI dependencies if needed
if [ ! -d "$SCRIPT_DIR/node_modules" ]; then
  echo -e "${YELLOW}Installing UI dependencies...${NC}"
  cd "$SCRIPT_DIR" || exit 1
  npm install
  cd "$SCRIPT_DIR" || exit 1
fi

# Build backend if needed
if [ ! -f "$BACKEND_PATH" ]; then
  echo -e "${YELLOW}Server binary not found, building first...${NC}"
  cargo build --release
fi

echo -e "${GREEN}Using backend at:${NC} $BACKEND_PATH"

# Create logs directory if it doesn't exist
mkdir -p "$SCRIPT_DIR/logs"
LOG_FILE="$SCRIPT_DIR/logs/backend-$(date +%Y%m%d-%H%M%S).log"
echo -e "${YELLOW}Backend logs will be saved to:${NC} $LOG_FILE"

# Start the server in the background with logging (suppress terminal output)
"$BACKEND_PATH" > "$LOG_FILE" 2>> "$LOG_FILE" &
SERVER_PID=$!

# Give server a moment to start
sleep 1

# Start UI with backend path as environment variable
echo -e "\n${BLUE}=== Starting UI client ===${NC}"
cd "$SCRIPT_DIR" || exit 1
BACKEND_BIN_PATH="$BACKEND_PATH" npm run build && BACKEND_BIN_PATH="$BACKEND_PATH" npm run start
UI_EXIT_CODE=$?

# Kill the server when the UI exits
if [ -n "$SERVER_PID" ]; then
  kill $SERVER_PID 2>/dev/null
  echo -e "${GREEN}Server process terminated.${NC}"
fi

echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}oli exited with code:${NC} $UI_EXIT_CODE"
echo -e "${BLUE}======================================${NC}"

exit $UI_EXIT_CODE
