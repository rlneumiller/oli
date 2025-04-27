#!/bin/bash
set -e

# Color codes
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Build script for oli (Rust backend + React/Ink frontend)
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Building oli (Hybrid Rust/React Application)${NC}"
echo -e "${BLUE}======================================${NC}"

# Check for required tools
command -v cargo >/dev/null 2>&1 || { echo -e "${RED}Error: cargo is not installed. Please install Rust.${NC}"; exit 1; }
command -v npm >/dev/null 2>&1 || { echo -e "${RED}Error: npm is not installed. Please install Node.js.${NC}"; exit 1; }

# Get the script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Clean previous build
echo -e "\n${BLUE}=== Cleaning previous build ===${NC}"
if [ -d "dist" ]; then
  echo -e "${YELLOW}Removing previous build in dist/ directory...${NC}"
  rm -rf dist/
fi
echo -e "${GREEN}Clean complete!${NC}"

# Build the Rust backend
echo -e "\n${BLUE}=== Building Rust backend ===${NC}"
cargo build --release

# Build the React/Ink frontend
echo -e "\n${BLUE}=== Building React frontend ===${NC}"

if [ ! -d "$SCRIPT_DIR/app" ]; then
  echo -e "${RED}Error: App directory not found${NC}"
  exit 1
fi

# Navigate to the App directory
cd "$SCRIPT_DIR" || exit 1
echo -e "${YELLOW}Installing npm dependencies...${NC}"
npm install
echo -e "${YELLOW}Building UI...${NC}"
npm run build
cd "$SCRIPT_DIR" || exit 1

# Create the single executable that combines both
echo -e "\n${BLUE}=== Creating combined cli package ===${NC}"

# Create the package directory
mkdir -p dist/oli

# Copy the binaries and UI files
cp target/release/oli-server dist/oli/ 2>/dev/null || echo -e "${YELLOW}Warning: oli-server binary not found${NC}"
mkdir -p dist/oli/app/dist

# Copy UI build output
if [ -d "app/dist" ]; then
  cp -r app/dist/* dist/oli/app/dist/ 2>/dev/null || echo -e "${YELLOW}Warning: UI dist files not found${NC}"
else
  echo -e "${YELLOW}Warning: UI dist directory not found${NC}"
fi

# Copy node_modules for dependencies
mkdir -p dist/oli/node_modules
cp -r node_modules dist/oli/ 2>/dev/null || echo -e "${YELLOW}Warning: node_modules not found${NC}"

# Create the wrapper script
echo -e "${YELLOW}Creating wrapper script...${NC}"
cat > dist/oli/oli << 'EOF'
#!/bin/bash

# Find the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Create logs directory if it doesn't exist
mkdir -p "$SCRIPT_DIR/logs"
LOG_FILE="$SCRIPT_DIR/logs/backend-$(date +%Y%m%d-%H%M%S).log"

# Check for print flag
if [[ ! "$*" == *"--print"* && ! "$*" == *"-p"* ]]; then
  echo "Backend logs will be saved to: $LOG_FILE"
fi

# Start the server in the background with logging
if [ -f "$SCRIPT_DIR/oli-server" ]; then
  "$SCRIPT_DIR/oli-server" > "$LOG_FILE" 2>&1 &
  SERVER_PID=$!

  # Give server a moment to start
  sleep 1

  # Start the UI
  cd "$SCRIPT_DIR"
  NODE_PATH="$SCRIPT_DIR/node_modules" node --import tsx app/dist/cli.js "$@"
  UI_EXIT_CODE=$?

  # Kill the server when the UI exits
  kill $SERVER_PID 2>/dev/null
  exit $UI_EXIT_CODE
else
  echo "Error: oli-server binary not found!"
  exit 1
fi
EOF

chmod +x dist/oli/oli

echo -e "\n${BLUE}======================================${NC}"
echo -e "${GREEN}=== Build complete! ===${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Binary is available at:${NC} ./dist/oli/oli"
echo -e "${GREEN}To run the application, use:${NC} './dist/oli/oli'"
