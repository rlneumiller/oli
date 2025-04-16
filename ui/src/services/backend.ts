import { spawn, ChildProcess } from "child_process";
import readline from "readline";
import { EventEmitter } from "events";

// JSON-RPC request ID counter
let requestId = 1;

// Interface for JSON-RPC request
interface JsonRpcRequest {
  jsonrpc: string;
  id: number;
  method: string;
  params: Record<string, unknown>;
}

// Interface for JSON-RPC response
interface JsonRpcResponse {
  jsonrpc: string;
  id: number;
  result?: Record<string, unknown>;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

// Interface for JSON-RPC notification
interface JsonRpcNotification {
  jsonrpc: string;
  method: string;
  params: Record<string, unknown>;
}

// Backend service for communication with the Rust backend
export class BackendService extends EventEmitter {
  private process: ChildProcess;
  private rl: readline.Interface;
  private pendingRequests: Map<
    number,
    {
      resolve: (value: Record<string, unknown>) => void;
      reject: (reason: Error) => void;
    }
  >;

  constructor(process: ChildProcess) {
    super();
    this.process = process;
    this.pendingRequests = new Map();

    // Create readline interface to read line by line from stdout
    this.rl = readline.createInterface({
      input: this.process.stdout!,
      crlfDelay: Infinity,
    });

    // Handle responses and notifications from the backend
    this.rl.on("line", (line) => {
      if (!line.trim()) {
        return;
      }

      try {
        const message = JSON.parse(line);

        // Handle JSON-RPC response
        if ("id" in message && message.id !== null) {
          const response = message as JsonRpcResponse;
          const pending = this.pendingRequests.get(response.id);

          if (pending) {
            if (response.error) {
              pending.reject(new Error(response.error.message));
            } else {
              pending.resolve(response.result);
            }
            this.pendingRequests.delete(response.id);
          }
        }
        // Handle JSON-RPC notification
        else if ("method" in message) {
          const notification = message as JsonRpcNotification;
          this.emit(notification.method, notification.params);
        }
      } catch {
        // Skip non-JSON messages silently to avoid polluting stdout
        if (!line.trim().startsWith("{")) {
          return; // Not JSON, likely debug output from the backend
        }
      }
    });

    // Handle errors (silently to avoid stdout pollution)
    this.process.stderr?.on("data", () => {
      // Error handling is done via events
    });

    // Handle process exit (silently)
    this.process.on("exit", () => {
      // Exit handling is done via events
    });
  }

  // Send a request to the backend
  async call(
    method: string,
    params: Record<string, unknown> = {},
  ): Promise<Record<string, unknown>> {
    const id = requestId++;
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      // Store the promise callbacks for later resolution
      this.pendingRequests.set(id, { resolve, reject });

      // Send the request to the backend
      this.process.stdin!.write(JSON.stringify(request) + "\n");
    });
  }

  // Kill the backend process
  kill() {
    this.process.kill();
  }

  // Emit a custom event
  emitEvent(event: string, data: Record<string, unknown> = {}) {
    return super.emit(event, data);
  }
}

// Spawn a new backend process
export function spawnBackend(path: string): BackendService {
  // Create process without logging to stdout
  const process = spawn(path, [], {
    stdio: ["pipe", "pipe", "pipe"],
  });

  // Create the backend service
  const backend = new BackendService(process);

  // Perform a health check silently
  setTimeout(async () => {
    try {
      // Try to call a simple method on the backend
      const result = await backend.call("get_available_models");

      // Success event
      backend.emitEvent("backend_connected", {
        success: true,
        message: "Successfully connected to backend",
        models: result.models,
      });
    } catch (error) {
      // Failure event
      backend.emitEvent("backend_connection_error", {
        success: false,
        message: "Failed to connect to backend server",
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }, 1000);

  return backend;
}
