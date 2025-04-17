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
// Try multiple potential backend paths
const potentialPaths = [
  path.resolve(__dirname, "../../oli-server"),
  path.resolve(process.cwd(), "../oli-server"),
  path.resolve(process.cwd(), "oli-server"),
  "/Users/amritkrishnan/src/oli/target/release/oli-server",
];
let backendPath = potentialPaths[0];
for (const p of potentialPaths) {
  console.log(`Checking backend path: ${p}`);
  try {
    const { accessSync, constants } = await import("fs");
    accessSync(p, constants.X_OK);
    backendPath = p;
    console.log(`Using backend at: ${backendPath}`);
    break;
  } catch {
    console.log(`Backend not found at: ${p}`);
  }
}

// Launch the Rust backend as a child process
const backend = spawnBackend(backendPath);

// Create the app element
const app = React.createElement(App, { backend: backend });

// Render the React app with custom options to avoid border rendering issues
const { waitUntilExit } = render(app, {
  // Disable console patching
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
