import { spawn, ChildProcess } from "child_process";
import readline from "readline";
import { EventEmitter } from "events";
import {
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcNotification,
} from "../types/index.js";

// Subscription tracking
type SubscriptionId = number;

// JSON-RPC request ID counter
let requestId = 1;

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

  // Method to check backend connection health
  checkConnection!: () => Promise<boolean>;

  // Track active subscriptions
  private subscriptions: Map<string, SubscriptionId> = new Map();

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
              pending.resolve(response.result || {});
            }
            this.pendingRequests.delete(response.id);
          }
        }
        // Handle JSON-RPC notification
        else if ("method" in message) {
          const notification = message as JsonRpcNotification;
          // Emit the notification event without logging to console
          this.emit(notification.method, notification.params);
        }
      } catch {
        // Skip non-JSON messages silently to avoid polluting stdout
        if (!line.trim().startsWith("{")) {
          return; // Not JSON, likely debug output from the backend
        }
      }
    });

    // Handle errors with better visibility
    this.process.stderr?.on("data", (data) => {
      // Log backend errors to console for debugging
      console.error(`Backend error: ${data.toString().trim()}`);
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

  // Subscribe to an event type
  async subscribe(eventType: string): Promise<SubscriptionId> {
    // Only subscribe once per event type
    if (this.subscriptions.has(eventType)) {
      return this.subscriptions.get(eventType) as SubscriptionId;
    }

    try {
      const result = await this.call("subscribe", { event_type: eventType });
      const subId = result.subscription_id as SubscriptionId;

      // Save subscription
      this.subscriptions.set(eventType, subId);
      return subId;
    } catch (error) {
      console.error(`Failed to subscribe to ${eventType}:`, error);
      throw error;
    }
  }

  // Unsubscribe from an event type
  async unsubscribe(eventType: string): Promise<boolean> {
    if (!this.subscriptions.has(eventType)) {
      return false; // Not subscribed
    }

    const subId = this.subscriptions.get(eventType) as SubscriptionId;
    try {
      const result = await this.call("unsubscribe", {
        event_type: eventType,
        subscription_id: subId,
      });

      const success = result.success as boolean;
      if (success) {
        this.subscriptions.delete(eventType);
      }

      return success;
    } catch (error) {
      console.error(`Failed to unsubscribe from ${eventType}:`, error);
      throw error;
    }
  }
}

// Spawn a new backend process
export function spawnBackend(path: string): BackendService {
  // Create process with debugging on stderr
  const process = spawn(path, [], {
    stdio: ["pipe", "pipe", "inherit"],
    detached: false
  });

  // Check if the process started correctly
  if (!process.pid) {
    console.error("Failed to start backend process");
    throw new Error("Failed to start backend process");
  }


  // Handle process errors
  process.on("error", (err) => {
    console.error("Backend process error:", err);
  });

  // Create the backend service
  const backend = new BackendService(process);

  // Perform a health check function
  const checkConnection = async () => {
    try {
      // Try to call a simple method on the backend with a generous timeout
      const result = await Promise.race([
        backend.call("get_available_models"),
        new Promise<Record<string, unknown>>((_, reject) =>
          setTimeout(() => reject(new Error("Connection timeout")), 10000)
        )
      ]) as Record<string, unknown>;

      // Get the version from the backend
      let version;
      try {
        const versionResult = await backend.call("get_version");
        version = versionResult.version as string;
      } catch (err) {
        // Continue anyway if version can't be retrieved
      }

      // Success event - emit with models data
      backend.emitEvent("backend_connected", {
        success: true,
        message: "Successfully connected to backend",
        models: result.models,
        version,
      });

      return true;
    } catch (error) {
      console.error("Backend connection check failed:",
        error instanceof Error ? error.message : String(error));

      // Notify of error but don't retry - let the CLI handle retries
      backend.emitEvent("backend_connection_error", {
        success: false,
        message: "Failed to connect to backend server",
        error: error instanceof Error ? error.message : String(error),
      });

      return false;
    }
  };

  // Create convenience method on the service to check connection
  backend.checkConnection = checkConnection;

  // Add a connection flag to track state
  (backend as any).isConnected = false;

  // Listen for connection events to update the flag
  backend.on("backend_connected", () => {
    (backend as any).isConnected = true;
  });

  // CRITICAL: Start the connection check automatically with a slight delay
  // This ensures the backend process is fully started before we try to connect
  setTimeout(() => {
    checkConnection().catch(err => {
      console.error("Initial connection check failed:", err);
    });
  }, 1000);

  return backend;
}
