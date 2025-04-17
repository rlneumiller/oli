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
      console.log(`Subscribed to ${eventType} events with ID ${subId}`);
      return subId;
    } catch (error) {
      console.error(`Failed to subscribe to ${eventType}:`, error);
      throw error;
    }
  }
  
  // Unsubscribe from an event type
  async unsubscribe(eventType: string): Promise<boolean> {
    if (!this.subscriptions.has(eventType)) {
      console.log(`Not subscribed to ${eventType}, nothing to unsubscribe`);
      return false; // Not subscribed
    }
    
    const subId = this.subscriptions.get(eventType) as SubscriptionId;
    try {
      const result = await this.call("unsubscribe", { 
        event_type: eventType, 
        subscription_id: subId 
      });
      
      const success = result.success as boolean;
      if (success) {
        this.subscriptions.delete(eventType);
        console.log(`Successfully unsubscribed from ${eventType}`);
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

      // Get the version from the backend
      let version;
      try {
        const versionResult = await backend.call("get_version");
        version = versionResult.version as string;
      } catch {
        // Don't set version if we can't get it from backend
        console.error("Failed to get version from backend");
      }

      // Success event
      backend.emitEvent("backend_connected", {
        success: true,
        message: "Successfully connected to backend",
        models: result.models,
        version,
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
