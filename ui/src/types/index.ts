// Common application types

// Message interface
export interface Message {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
  task_id?: string;
  tool?: string;
  tool_status?: ToolStatus;
  tool_data?: ToolData;
}

// Message role type
export type MessageRole = "user" | "assistant" | "system" | "tool";

// Tool status enum
export type ToolStatus = "running" | "success" | "error";

// Tool data interface
export interface ToolData {
  name: string;
  file_path?: string;
  lines?: number;
  description?: string;
}

// Tool execution interface
export interface ToolExecution {
  id: string;
  task_id: string;
  name: string;
  status: ToolStatus;
  startTime: number;
  endTime?: number;
  message: string;
  metadata: Record<string, any>;
}

// Tool status update interface
export interface ToolStatusUpdate {
  type: "started" | "updated";
  execution: ToolExecution;
}

// Model interface
export interface Model {
  name: string;
  id: string;
  description: string;
  supports_agent: boolean;
}

// Task interface
export interface Task {
  id: string;
  name: string;
  status: "pending" | "running" | "complete" | "error";
  tool_count?: number;
}

// App state interface
export interface AppState {
  models: Model[];
  selectedModel: number;
  messages: Message[];
  tasks: Task[];
  isProcessing: boolean;
  error: string | null;
  backendConnected: boolean;
  appMode: "setup" | "chat";
  useAgent: boolean;
  backendInfo?: Record<string, unknown>; // Contains backend-related info including version
}

// Available commands
export interface Command {
  name: string;
  description: string;
  value: string;
}

// JSON-RPC request interface
export interface JsonRpcRequest {
  jsonrpc: string;
  id: number;
  method: string;
  params: Record<string, unknown>;
}

// JSON-RPC response interface
export interface JsonRpcResponse {
  jsonrpc: string;
  id: number;
  result?: Record<string, unknown>;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

// JSON-RPC notification interface
export interface JsonRpcNotification {
  jsonrpc: string;
  method: string;
  params: Record<string, unknown>;
}
