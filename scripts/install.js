#!/usr/bin/env node

import axios from 'axios';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { execSync } from 'child_process';
import tar from 'tar';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

// Set up require for ES modules
const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configuration
const GITHUB_REPO = 'amrit110/oli';
const BINARY_NAME = 'oli';
const packageJson = require('../package.json');
const PKG_VERSION = packageJson.version;

async function install() {
  try {
    console.log('Running oli setup...');

    // If we're running in development mode, just create necessary directories
    if (process.env.NODE_ENV === 'development' || fs.existsSync(path.join(__dirname, '..', 'app'))) {
      console.log('Development environment detected, skipping download of pre-built binaries');

      // Create the bin directory if it doesn't exist
      const binDir = path.join(__dirname, '..', 'bin');
      if (!fs.existsSync(binDir)) {
        fs.mkdirSync(binDir, { recursive: true });
      }

      // Create logs directory if it doesn't exist
      const logsDir = path.join(__dirname, '..', 'logs');
      if (!fs.existsSync(logsDir)) {
        fs.mkdirSync(logsDir, { recursive: true });
      }

      // Ensure app directory exists
      if (!fs.existsSync(path.join(__dirname, '..', 'app'))) {
        console.log('Warning: app directory not found');
      }

      console.log('Setup complete');
      return;
    }

    // If we're here, we need to install from a release
    console.log('Installing oli from release binaries...');

    // Determine OS and architecture
    const platform = os.platform();
    const arch = os.arch();

    if (!(platform === 'darwin' || platform === 'linux')) {
      throw new Error(`Unsupported platform: ${platform}. oli supports macOS and Linux.`);
    }

    if (!(arch === 'x64' || arch === 'arm64')) {
      throw new Error(`Unsupported architecture: ${arch}. oli supports x64 and arm64.`);
    }

    // Create the bin directory if it doesn't exist
    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    // Download URL for the tarball
    const platform_suffix = platform === 'darwin' ? 'macos' : 'linux';
    const arch_suffix = arch === 'x64' ? 'x86_64' : 'aarch64';
    const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${PKG_VERSION}/oli-${PKG_VERSION}-${platform_suffix}-${arch_suffix}.tar.gz`;
    console.log(`Downloading from: ${downloadUrl}`);

    // Create temp directory
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'oli-'));
    const tarballPath = path.join(tmpDir, 'oli.tar.gz');

    // Download the tarball
    const response = await axios.get(downloadUrl, { responseType: 'arraybuffer' });
    fs.writeFileSync(tarballPath, Buffer.from(response.data));

    // Extract the tarball
    await tar.extract({
      file: tarballPath,
      cwd: tmpDir
    });

    // Copy files to bin directory
    fs.copyFileSync(path.join(tmpDir, 'oli', 'oli'), path.join(binDir, 'oli'));
    fs.chmodSync(path.join(binDir, 'oli'), 0o755); // Make executable

    // Copy the server and app files
    const libexecDir = path.join(__dirname, '..', 'libexec');
    if (!fs.existsSync(libexecDir)) {
      fs.mkdirSync(libexecDir, { recursive: true });
    }

    fs.copyFileSync(path.join(tmpDir, 'oli', 'oli-server'), path.join(libexecDir, 'oli-server'));
    fs.chmodSync(path.join(libexecDir, 'oli-server'), 0o755); // Make executable

    // Copy app directory
    const appDir = path.join(libexecDir, 'app');
    if (!fs.existsSync(appDir)) {
      fs.mkdirSync(appDir, { recursive: true });
    }

    // Copy app files recursively
    copyDirSync(path.join(tmpDir, 'oli', 'app'), appDir);

    // Copy node_modules
    const nodeModulesDir = path.join(libexecDir, 'node_modules');
    if (!fs.existsSync(nodeModulesDir)) {
      fs.mkdirSync(nodeModulesDir, { recursive: true });
    }
    copyDirSync(path.join(tmpDir, 'oli', 'node_modules'), nodeModulesDir);

    // Update the bin script to point to the correct paths
    const binScript = `#!/bin/bash
# Find the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PARENT_DIR="$( dirname "$SCRIPT_DIR" )"

# Create logs directory if it doesn't exist
mkdir -p "$PARENT_DIR/logs"
LOG_FILE="$PARENT_DIR/logs/backend-$(date +%Y%m%d-%H%M%S).log"

# Check for print flag
if [[ ! "$*" == *"--print"* && ! "$*" == *"-p"* ]]; then
  echo "Backend logs will be saved to: $LOG_FILE"
fi

# Start the server in the background with logging
"$PARENT_DIR/libexec/oli-server" > "$LOG_FILE" 2>&1 &
SERVER_PID=$!

# Give server a moment to start
sleep 1

# Start the UI
cd "$PARENT_DIR/libexec"
NODE_PATH="$PARENT_DIR/libexec/node_modules" node --import tsx app/dist/cli.js "$@"
UI_EXIT_CODE=$?

# Kill the server when the UI exits
kill $SERVER_PID 2>/dev/null
exit $UI_EXIT_CODE
`;

    fs.writeFileSync(path.join(binDir, 'oli'), binScript);
    fs.chmodSync(path.join(binDir, 'oli'), 0o755); // Make executable

    // Clean up
    fs.rmSync(tmpDir, { recursive: true, force: true });

    console.log('oli has been installed successfully!');
    console.log('You can now run it using the "oli" command.');

  } catch (error) {
    console.error('Installation failed:', error.message);
    process.exit(1);
  }
}

// Helper function to copy directories recursively
function copyDirSync(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  const entries = fs.readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDirSync(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

// Set development environment
process.env.NODE_ENV = 'development';

// Run the installation
install().catch(console.error);
