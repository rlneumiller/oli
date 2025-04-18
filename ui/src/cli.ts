#!/usr/bin/env node
import React from "react";
import { render } from "ink";
import App from "./components/App.js";
import path from "path";
import { fileURLToPath } from "url";
import { spawnBackend } from "./services/backend.js";

// CLI arguments are available if needed
// process.argv.slice(2);

// Setup environment for the app
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
console.log(`Current directory: ${process.cwd()}`);
console.log(`Script directory: ${__dirname}`);
// Check for environment variable first
const envBackendPath = process.env.BACKEND_BIN_PATH;
if (envBackendPath) {
  console.log(`Environment provided backend path: ${envBackendPath}`);
}

// Try multiple potential backend paths
const potentialPaths = [
  // First try the environment variable if provided
  ...(envBackendPath ? [envBackendPath] : []),
  path.resolve(__dirname, "../../target/release/oli-server"), // Path relative to script dir
  path.resolve(process.cwd(), "../target/release/oli-server"), // Path relative to current dir
  path.resolve(process.cwd(), "oli-server"), // Local path
  path.resolve(process.cwd(), "../oli-server"), // Sibling directory
];
// Determine env-specific paths
const isProduction = process.env.NODE_ENV === "production";
const rootDir = path.resolve(__dirname, "../../"); // Project root directory

// Add additional paths based on environment
if (isProduction) {
  // Add installation-specific paths for production
  potentialPaths.push(
    path.join(rootDir, "bin/oli-server"),
    "/usr/local/bin/oli-server",
  );
}

let backendPath = potentialPaths[0];
let backendFound = false;

for (const p of potentialPaths) {
  console.log(`Checking backend path: ${p}`);
  try {
    const { accessSync, constants } = await import("fs");
    accessSync(p, constants.X_OK);
    backendPath = p;
    backendFound = true;
    console.log(`Using backend at: ${backendPath}`);
    break;
  } catch {
    console.log(`Backend not found at: ${p}`);
  }
}

if (!backendFound) {
  console.error(
    "ERROR: Could not find oli-server binary. Please build with './build.sh' first.",
  );
}

// Launch the Rust backend as a child process
const backend = spawnBackend(backendPath);

// When dealing with raw terminal UIs it's helpful to completely
// disable the default React development warnings
process.env.NODE_ENV = "production";

// Create the app element without a header - we'll draw it manually
const app = React.createElement(App, {
  backend: backend,
});

// Clear the terminal before rendering anything
process.stdout.write("\x1B[2J\x1B[H\x1B[J");

// Clear the terminal before rendering

// Render the React app with custom options
const { waitUntilExit } = render(app, {
  // Disable console patching to avoid interference
  patchConsole: false,
  // Use standard IO
  stdin: process.stdin,
  stdout: process.stdout,
  stderr: process.stderr,
  // Allow Ctrl+C to exit
  exitOnCtrlC: true,
});

// Handle graceful shutdown
const cleanup = () => {
  backend.kill();
  process.exit(0);
};

// Register signal handlers
process.on("SIGINT", cleanup);
process.on("SIGTERM", cleanup);

// Wait for the app to exit
waitUntilExit().then(cleanup);
