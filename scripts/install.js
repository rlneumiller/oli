#!/usr/bin/env node

const axios = require('axios');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');
const tar = require('tar');

// Configuration
const GITHUB_REPO = 'amrit110/oli';
const BINARY_NAME = 'oli';
const PKG_VERSION = require('../package.json').version;

async function install() {
  try {
    console.log('Installing Oli...');
    
    // Determine OS and architecture
    const platform = os.platform();
    const arch = os.arch();
    
    if (!(platform === 'darwin' || platform === 'linux')) {
      throw new Error(`Unsupported platform: ${platform}. Oli supports macOS and Linux.`);
    }
    
    if (!(arch === 'x64' || arch === 'arm64')) {
      throw new Error(`Unsupported architecture: ${arch}. Oli supports x64 and arm64.`);
    }
    
    // Create the bin directory if it doesn't exist
    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
    
    // Download URL for the tarball
    const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${PKG_VERSION}/oli-${PKG_VERSION}.tar.gz`;
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
    
    // Copy the server and UI files
    const libexecDir = path.join(__dirname, '..', 'libexec');
    if (!fs.existsSync(libexecDir)) {
      fs.mkdirSync(libexecDir, { recursive: true });
    }
    
    fs.copyFileSync(path.join(tmpDir, 'oli', 'oli-server'), path.join(libexecDir, 'oli-server'));
    fs.chmodSync(path.join(libexecDir, 'oli-server'), 0o755); // Make executable
    
    // Copy UI directory
    const uiDir = path.join(libexecDir, 'ui');
    if (!fs.existsSync(uiDir)) {
      fs.mkdirSync(uiDir, { recursive: true });
    }
    
    // Copy UI files recursively
    copyDirSync(path.join(tmpDir, 'oli', 'ui'), uiDir);
    
    // Update the bin script to point to the correct paths
    const binScript = `#!/bin/bash
# Find the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PARENT_DIR="$( dirname "$SCRIPT_DIR" )"

# Start the server in the background
"$PARENT_DIR/libexec/oli-server" &
SERVER_PID=$!

# Start the UI
cd "$PARENT_DIR/libexec"
node --import tsx ui/cli.js "$@"

# Kill the server when the UI exits
kill $SERVER_PID
`;
    
    fs.writeFileSync(path.join(binDir, 'oli'), binScript);
    fs.chmodSync(path.join(binDir, 'oli'), 0o755); // Make executable
    
    // Clean up
    fs.rmSync(tmpDir, { recursive: true, force: true });
    
    console.log('Oli has been installed successfully!');
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

// Run the installation
install().catch(console.error);