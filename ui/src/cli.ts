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
  noHeader: true, // Pass prop to disable header in App component
});

// Clear the terminal before rendering anything
process.stdout.write("\x1B[2J\x1B[H\x1B[J");

// Render fixed header outside of React/Ink's render cycle
const modelName = "Claude 3.7 Sonnet"; // Hardcoded for now - will be updated once loaded

// Variables for precise box drawing with exact character counts
const totalWidth = 36; // Total width including borders
const innerWidth = totalWidth - 2; // Inner width (accounting for side borders)
const content = ` oli • ${modelName}`; // Add leading space for padding from left border
const paddingNeeded = innerWidth - content.length;
const padding = " ".repeat(paddingNeeded);

// Draw the perfectly aligned header
// The box-drawing is precisely calculated for alignment
const topBorder = "┌" + "─".repeat(innerWidth) + "┐";
const contentLine = "│" + content + padding + "│";
const bottomBorder = "└" + "─".repeat(innerWidth) + "┘";

console.log(
  "\x1B[32m" + topBorder + "\n" + contentLine + "\n" + bottomBorder + "\x1B[0m",
);

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
