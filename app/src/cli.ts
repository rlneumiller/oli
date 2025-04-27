#!/usr/bin/env node
import React from "react";
import { render } from "ink";
import App from "./components/App.js";
import path from "path";
import { fileURLToPath } from "url";
import { spawnBackend } from "./services/backend.js";
import fs from "fs";
import { createRequire } from "module";
import { BackendService } from "./services/backend.js";

// Parse command line arguments
const args = [...process.argv.slice(2)];

// Get __dirname equivalent in ESM
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Setup require for package.json
const require = createRequire(import.meta.url);

// Check for help flag
if (args.includes("--help") || args.includes("-h")) {
  console.log(`
Usage: oli [options] [prompt]

oli - starts an interactive session with model selection by default

Arguments:
  prompt                          Your prompt (requires -m/--model if specified)

Options:
  -p, --print                     Print output to stdout (requires -m/--model)
  -m, --model <name>              Select a model by name or ID
  -l, --list                      List all available models
  -h, --help                      Show this help message
  -v, --version                   Show version information

Examples:
  oli                             Start interactive session with model selection
  oli -m gpt-4o                   Start interactive session with specified model
  oli -m gpt-4o "What is TypeScript?"   Run query with specified model
  oli -m gpt-4o -p "Hello world"        Run in non-interactive mode
  oli -l                          List all available models
  `);
  process.exit(0);
}

// Check for version flag
if (args.includes("--version") || args.includes("-v")) {
  const packageJsonPath = path.resolve(__dirname, "../../../package.json");
  const packageJson = require(packageJsonPath);
  console.log(`oli v${packageJson.version}`);
  process.exit(0);
}

// Parse command line args
let printMode = false;
let listModels = false;
let selectedModelName: string | null = null;
let prompt = "";

// Process arguments
for (let i = 0; i < args.length; i++) {
  const arg = args[i];

  if (arg === "--print" || arg === "-p") {
    printMode = true;
  } else if (arg === "--list" || arg === "-l") {
    listModels = true;
  } else if (arg === "--model" || arg === "-m") {
    if (i + 1 < args.length) {
      selectedModelName = args[i + 1];
      i++; // Skip the next argument as it's the model name
    } else {
      console.error("Error: Model name is required with -m/--model flag");
      process.exit(1);
    }
  } else {
    // If it's not a flag or a flag value, it's part of the prompt
    prompt = prompt ? `${prompt} ${arg}` : arg;
  }
}

// Setup environment for the app
console.log(`Current directory: ${process.cwd()}`);
console.log(`Script directory: ${__dirname}`);

// Check for environment variable first
const envBackendPath = process.env.BACKEND_BIN_PATH;
if (envBackendPath) {
  console.log(`Environment provided backend path: ${envBackendPath}`);
}

// First check if BACKEND_BIN_PATH is set - this is crucial
const potentialPaths = [];

// Always prioritize environment variable if provided
if (envBackendPath) {
  console.log(`Using environment-provided backend path: ${envBackendPath}`);
  potentialPaths.push(envBackendPath);
}

// When running from dist/oli directory, check the local directory first
// This is the most likely location in the packaged version
potentialPaths.push(path.resolve(process.cwd(), "oli-server"));

// Check current script directory - especially useful in distribution
const scriptDir = path.dirname(process.argv[1]);
potentialPaths.push(path.resolve(scriptDir, "oli-server"));

// For development: Check relative to app/dist location
potentialPaths.push(path.resolve(__dirname, "../../oli-server"));

// Last resort - try standard development build locations
potentialPaths.push(
  path.resolve(process.cwd(), "../target/release/oli-server"),
  path.resolve(__dirname, "../../../target/release/oli-server")
);

let backendPath = potentialPaths[0];
let backendFound = false;

for (const p of potentialPaths) {
  console.log(`Checking backend path: ${p}`);
  try {
    const { accessSync, constants } = fs;
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
  process.exit(1);
}

// Launch the Rust backend as a child process
const backend = spawnBackend(backendPath);

// Define model interface
interface Model {
  name: string;
  id: string;
  description?: string;
  supports_agent: boolean;
}

// Function to handle model selection
async function selectModel(backend: BackendService, modelName: string): Promise<number> {
  try {
    // Get available models
    const result = await backend.call("get_available_models");
    const models = result.models as Model[] || [];

    if (!models.length) {
      console.error("Error: No models available");
      backend.kill();
      process.exit(1);
    }

    // Find the requested model by name or ID
    const modelIndex = models.findIndex(
      (m: Model) => m.name.toLowerCase() === modelName.toLowerCase() ||
                 m.id.toLowerCase() === modelName.toLowerCase()
    );

    if (modelIndex === -1) {
      console.error(`Error: Model "${modelName}" not found. Available models are:`);
      formatAndPrintModels(models);
      backend.kill();
      process.exit(1);
    }

    // Set the selected model
    await backend.call("set_selected_model", { model_index: modelIndex });
    return modelIndex;
  } catch (error) {
    console.error("Error selecting model:", error instanceof Error ? error.message : String(error));
    backend.kill();
    process.exit(1);
  }
}

// Function to format and print models
function formatAndPrintModels(models: Model[]): void {
  console.log("\nAvailable Models:");
  console.log("─".repeat(60));
  console.log("Index | Name                 | ID                 | Agent Support");
  console.log("─".repeat(60));

  models.forEach((model, index) => {
    const name = model.name.padEnd(20);
    const id = model.id.padEnd(18);
    const agentSupport = model.supports_agent ? "Yes" : "No";
    console.log(`${String(index).padEnd(5)} | ${name} | ${id} | ${agentSupport}`);
  });

  console.log("─".repeat(60));
}

// Main execution flow based on args
async function main() {
  try {
    // Wait for backend connection

    // Start connection check explicitly - this is crucial for interactive mode
    backend.checkConnection().catch(() => {
      // Silently continue - we'll wait for the events below
    });

    // Create a promise that waits for the backend to connect
    await new Promise<void>((resolve, reject) => {
      // Set up timeout for connection
      const connectionTimeout = setTimeout(() => {
        // If timeout occurs, try one last connection check before giving up
        backend.checkConnection()
          .then(success => {
            if (success) {
              resolve();
            } else {
              reject(new Error("Connection timed out after final attempt"));
            }
          })
          .catch(() => {
            reject(new Error("Connection timed out"));
          });
      }, 10000); // 10 seconds initial timeout before final attempt

      // Set up event handlers for connection events
      const onConnected = (params: any) => {
        clearTimeout(connectionTimeout);
        if (params.models) {
          resolve();
        } else {
          // Still resolve but log a warning
            resolve();
        }
      };

      const onError = (params: any) => {
        // Error is handled via timeout
        // Don't reject on first error - just log it and wait for timeout or success
      };

      // Register for connection events
      backend.once("backend_connected", onConnected);
      backend.on("backend_connection_error", onError);

      // If backend is already connected, resolve immediately
      if ((backend as any).isConnected) {
        clearTimeout(connectionTimeout);
        backend.off("backend_connection_error", onError);
        resolve();
      }
    }).catch(error => {
      throw new Error("Could not connect to backend server. Please try again or check the logs.");
    });

    // Get available models
    const modelsResult = await backend.call("get_available_models");
    const models = modelsResult.models as Model[] || [];

    // List models if requested
    if (listModels) {
      formatAndPrintModels(models);
      backend.kill();
      process.exit(0);
    }

    // Check for required -m/--model flag for prompt and print mode
    if ((prompt || printMode) && !selectedModelName) {
      console.error("Error: The -m/--model flag is required when using a prompt or -p/--print mode");
      console.error("Run 'oli -l' to see available models");
      console.error("Example: oli -m gpt-4o \"What is TypeScript?\"");
      backend.kill();
      process.exit(1);
    }

    // Handle model selection if specified
    let selectedModelIndex: number | undefined = undefined;
    if (selectedModelName) {
      selectedModelIndex = await selectModel(backend, selectedModelName);
    }

    // Handle non-interactive mode (-p/--print)
    if (printMode) {
      if (!prompt) {
        console.error("Error: A prompt is required when using -p/--print mode");
        backend.kill();
        process.exit(1);
      }

      try {
        const result = await backend.call("run", {
          prompt,
          model_index: selectedModelIndex
        });
        console.log(result.response);
        backend.kill();
        process.exit(0);
      } catch (error) {
        console.error("Error:", error instanceof Error ? error.message : String(error));
        backend.kill();
        process.exit(1);
      }
    }

    // Handle interactive mode with model and prompt, model only, or default
    if (selectedModelIndex !== undefined) {
      // Show selected model
      if (models.length > 0 && selectedModelIndex < models.length) {
        const model = models[selectedModelIndex];
        console.log(`Using model: ${model.name}`);
      }

      // Start interactive mode with selected model (with or without prompt)
      startInteractiveMode(backend, prompt || null, selectedModelIndex);
    } else {
      // Start default interactive mode with model selection screen
      startInteractiveMode(backend);
    }
  } catch (error) {
    console.error("Error:", error instanceof Error ? error.message : String(error));
    backend.kill();
    process.exit(1);
  }
}

// Start main execution
main().catch(error => {
  console.error("Unhandled error:", error instanceof Error ? error.message : String(error));
  backend.kill();
  process.exit(1);
});

// Function to start interactive mode with optional model selection
function startInteractiveMode(
  backend: BackendService,
  initialPrompt: string | null = null,
  modelIndex?: number
): void {
  // When dealing with raw terminal UIs it's helpful to completely
  // disable the default React development warnings
  process.env.NODE_ENV = "production";

  // Create the app element with or without model selection
  const props: any = {
    backend: backend,
    initialPrompt: initialPrompt
  };

  // Only pass initialModelIndex if specified (undefined means show model selection)
  if (modelIndex !== undefined) {
    props.initialModelIndex = modelIndex;
  }

  const app = React.createElement(App, props);

  // Clear the terminal before rendering anything
  process.stdout.write("\x1B[2J\x1B[H\x1B[J");

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
}
