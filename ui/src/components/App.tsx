import React, { useEffect, useState, useCallback, useMemo } from "react";
import { Box, Text } from "ink";
import { BackendService } from "../services/backend.js";
import ChatInterface from "./ChatInterface.js";
import ModelSelector from "./ModelSelector.js";
import StatusBar from "./StatusBar.js";
import theme from "../styles/gruvbox.js";

import { AppState, ToolExecution, ToolStatusUpdate } from "../types/index.js";
import { isCommand } from "../utils/commandUtils.js";
import {
  executeCommand,
  processUserMessage,
} from "../utils/commandHandlers.js";

// App props interface
interface AppProps {
  backend: BackendService;
}

// Main app component
const App: React.FC<AppProps> = ({ backend }) => {
  // App state
  const [state, setState] = useState<AppState>({
    models: [],
    selectedModel: 0,
    messages: [],
    tasks: [],
    isProcessing: false,
    error: null,
    backendConnected: false,
    appMode: "setup", // Start in setup mode
    useAgent: true, // Agent mode is always enabled
  });
  
  // Tool executions state - separate to avoid re-rendering the entire app on tool updates
  const [toolExecutions, setToolExecutions] = useState<Map<string, ToolExecution>>(new Map());

  // UI state
  const [showShortcuts, setShowShortcuts] = useState(false);

  // Subscribe to tool status events
  useEffect(() => {
    // Setup tool status subscription when backend is available
    const setupToolStatusSubscription = async () => {
      try {
        await backend.subscribe("tool_status");
        console.log("Subscribed to tool status updates");
      } catch (error) {
        console.error("Failed to subscribe to tool status updates:", error);
      }
    };
    
    // Handle tool status events
    const handleToolStatus = (params: ToolStatusUpdate) => {
      // Log tool status events for debugging
      console.log(`Tool status update received: ${JSON.stringify(params)}`);
      
      const { type, execution } = params;
      
      setToolExecutions(prev => {
        // Create a new Map to avoid mutating state
        const newMap = new Map(prev);
        
        if (type === "started") {
          // Add new tool execution
          console.log(`Adding new tool execution: ${execution.id} (${execution.name})`);
          newMap.set(execution.id, execution);
        } else if (type === "updated") {
          // Update existing tool execution
          console.log(`Updating tool execution: ${execution.id} (${execution.status})`);
          newMap.set(execution.id, execution);
          
          // Clean up completed/errored tools after 30 seconds
          if (execution.status !== "running" && execution.endTime) {
            setTimeout(() => {
              setToolExecutions(current => {
                const updatedMap = new Map(current);
                updatedMap.delete(execution.id);
                return updatedMap;
              });
            }, 30000);
          }
        }
        
        // Log the state of tool executions after update
        console.log(`Tool executions count: ${newMap.size}`);
        
        return newMap;
      });
    };
    
    // Subscribe when component mounts
    backend.on("tool_status", handleToolStatus);
    setupToolStatusSubscription();
    
    // Unsubscribe when component unmounts
    return () => {
      backend.off("tool_status", handleToolStatus);
      backend.unsubscribe("tool_status").catch(console.error);
    };
  }, [backend]);

  // Load initial data
  useEffect(() => {
    // Listen for backend connection events
    backend.on("backend_connected", (params) => {
      setState((prev) => ({
        ...prev,
        models: params.models || [],
        backendConnected: true,
        backendInfo: {
          ...params,
        },
      }));
    });

    backend.on("backend_connection_error", (params) => {
      setState((prev) => ({
        ...prev,
        error: params.error,
        backendConnected: false,
        messages: [
          ...prev.messages,
          {
            id: `system-${Date.now()}`,
            role: "system",
            content: `Failed to connect to backend: ${params.error}`,
            timestamp: Date.now(),
          },
        ],
      }));
    });

    // Register event listeners for backend notifications
    backend.on("processing_started", (params) => {
      setState((prev) => ({
        ...prev,
        isProcessing: true,
        // If agent mode is specified in the event, update state
        ...(params.use_agent !== undefined
          ? { useAgent: params.use_agent }
          : {}),
      }));
    });

    backend.on("processing_progress", (params) => {
      // Add progress message if it's not already in the list
      setState((prev) => {
        // Only add the message if it's not a duplicate
        if (!prev.messages.some((m) => m.content === params.message)) {
          return {
            ...prev,
            messages: [
              ...prev.messages,
              {
                id: `progress-${Date.now()}`,
                role: "system",
                content: params.message,
                timestamp: Date.now(),
                task_id: params.task_id,
              },
            ],
          };
        }
        return prev;
      });
    });

    backend.on("processing_complete", () => {
      setState((prev) => ({
        ...prev,
        isProcessing: false,
      }));
    });

    backend.on("processing_error", (params) => {
      setState((prev) => ({
        ...prev,
        isProcessing: false,
        error: params.error,
        messages: [
          ...prev.messages,
          {
            id: `error-${Date.now()}`,
            role: "system",
            content: `Error: ${params.error}`,
            timestamp: Date.now(),
          },
        ],
      }));
    });

    backend.on("tool_execution", (params) => {
      // Generate a unique identifier for this tool execution
      const toolId = `tool-${params.tool}-${Date.now()}`;

      // Extract tool data from the enhanced tool execution event
      const toolData = {
        name: params.tool,
        file_path: params.file_path || undefined,
        lines: params.lines || undefined,
        description: params.description || undefined,
      };

      // Use the status provided by the backend, or fall back to our own detection
      const toolStatus: "running" | "success" | "error" =
        params.status || "running";

      // For running tools, check if we already have a message for this tool and update it
      if (toolStatus === "running") {
        console.log(`Tool running: ${params.tool} - ${params.message}`);
      } else if (toolStatus === "success") {
        console.log(`Tool completed: ${params.tool} - ${params.message}`);
      } else if (toolStatus === "error") {
        console.log(`Tool error: ${params.tool} - ${params.message}`);
      }

      // Bridge old tool_execution events to the new tool_status system
      // This allows legacy events to appear in the ToolStatusPanel
      setToolExecutions(prev => {
        const newMap = new Map(prev);
        const execution: ToolExecution = {
          id: toolId,
          task_id: params.task_id || "",
          name: params.tool,
          status: toolStatus,
          startTime: Date.now(),
          endTime: toolStatus !== "running" ? Date.now() : undefined,
          message: params.message,
          metadata: {
            file_path: params.file_path,
            lines: params.lines,
            description: params.description
          }
        };
        
        // Add to tool executions map
        newMap.set(toolId, execution);
        console.log(`Added legacy tool execution to map: ${toolId}, total=${newMap.size}`);
        
        return newMap;
      });

      setState((prev) => {
        // Check if we already have a tool message for this specific tool execution
        // If we need to update existing tools, we can implement this logic here
        // For now, just add a new message for each tool event

        return {
          ...prev,
          messages: [
            ...prev.messages,
            {
              id: toolId,
              role: "tool",
              content: `[${params.tool}] ${params.message}`,
              timestamp: Date.now(),
              task_id: params.task_id,
              tool: params.tool,
              tool_status: toolStatus,
              tool_data: toolData,
            },
          ],
          // Update task information if we have task_id
          tasks: prev.tasks.map((task) =>
            task.id === params.task_id
              ? { ...task, tool_count: (task.tool_count || 0) + 1 }
              : task,
          ),
        };
      });
    });

    backend.on("log_message", () => {
      // Silent log handling
    });

    // Clean up event listeners on component unmount
    return () => {
      backend.removeAllListeners();
    };
  }, [backend]);

  // Handle model selection - memoized to prevent unnecessary rerenders
  const handleModelSelect = useCallback((index: number) => {
    setState((prev) => ({
      ...prev,
      selectedModel: index,
    }));
  }, []);

  // Memoize the clear history handler
  const handleClearHistory = useCallback(() => {
    // Clear all messages from the UI state
    setState((prev) => ({
      ...prev,
      messages: [], // Clear all messages
      error: null, // Also clear any error state
    }));
  }, []);

  // Memoize command execution handler to reduce rerenders
  const handleExecuteCommand = useCallback(
    (command: string) => {
      // First try to execute as a built-in command
      const wasHandled = executeCommand(command, state, setState, backend, {
        handleClearHistory,
        handleModelSelect,
      });

      // If not a built-in command, handle as regular input
      if (!wasHandled) {
        processUserMessage(command, state, setState, backend);
      }
    },
    [state, backend, handleClearHistory, handleModelSelect],
  );

  // Handle regular user input (non-commands)
  const handleRegularInput = useCallback(
    async (input: string) => {
      // Process user message without command handling
      await processUserMessage(input, state, setState, backend);
    },
    [state, setState, backend],
  );

  // Combined handler for all user input
  const handleUserInput = useCallback(
    async (input: string) => {
      // If this is a command, handle it separately through the command handler
      if (isCommand(input)) {
        handleExecuteCommand(input);
        return;
      }

      // This is a regular user message - send it to the backend
      await handleRegularInput(input);
    },
    [handleExecuteCommand, handleRegularInput],
  );

  // Handle model confirmation and switch to chat mode - memoized to prevent unnecessary rerenders
  const handleModelConfirm = useCallback(() => {
    // Only proceed if we have models and backend is connected
    if (state.models.length > 0 && state.backendConnected) {
      setState((prev) => ({
        ...prev,
        appMode: "chat",
      }));
    }
  }, [state.models, state.backendConnected]);

  // Memoize the toggle shortcuts handler
  const handleToggleShortcuts = useCallback(() => {
    setShowShortcuts((prev) => !prev);
  }, []);

  // Memoize components to prevent unnecessary rerenders
  const modelSelectorComponent = useMemo(
    () => (
      <ModelSelector
        models={state.models}
        selectedIndex={state.selectedModel}
        onSelect={handleModelSelect}
        onConfirm={handleModelConfirm}
        isLoading={!state.backendConnected || state.models.length === 0}
      />
    ),
    [
      state.models,
      state.selectedModel,
      state.backendConnected,
      handleModelSelect,
      handleModelConfirm,
    ],
  );

  // Handle task interruption
  const handleInterrupt = useCallback(() => {
    // Call the backend to interrupt the current task
    if (state.isProcessing) {
      backend
        .call("interrupt_processing", {})
        .then(() => {
          setState((prev) => ({
            ...prev,
            isProcessing: false,
            messages: [
              ...prev.messages,
              {
                id: `system-${Date.now()}`,
                role: "system",
                content: "Task interrupted by user",
                timestamp: Date.now(),
              },
            ],
          }));
        })
        .catch((err) => {
          console.error("Failed to interrupt task:", err);
          // Set processing to false anyway to update UI
          setState((prev) => ({
            ...prev,
            isProcessing: false,
            messages: [
              ...prev.messages,
              {
                id: `system-${Date.now()}`,
                role: "system",
                content: "Attempted to interrupt task but encountered an error",
                timestamp: Date.now(),
              },
            ],
          }));
        });
    }
  }, [state.isProcessing, backend]);

  const chatInterfaceComponent = useMemo(
    () => (
      <ChatInterface
        messages={state.messages}
        isProcessing={state.isProcessing}
        onSubmit={handleUserInput}
        onInterrupt={handleInterrupt}
        showShortcuts={showShortcuts}
        onToggleShortcuts={handleToggleShortcuts}
        onClearHistory={handleClearHistory}
        onExecuteCommand={handleExecuteCommand}
        tasks={state.tasks}
        toolExecutions={toolExecutions}
      />
    ),
    [
      state.messages,
      state.isProcessing,
      state.tasks,
      toolExecutions,
      handleUserInput,
      handleInterrupt,
      showShortcuts,
      handleToggleShortcuts,
      handleClearHistory,
      handleExecuteCommand,
    ],
  );

  const statusBarComponent = useMemo(
    () => (
      <StatusBar
        modelName={state.models[state.selectedModel]?.name || "AI Assistant"}
        isProcessing={state.isProcessing}
        backendConnected={state.backendConnected}
        showShortcuts={showShortcuts}
      />
    ),
    [
      state.models,
      state.selectedModel,
      state.isProcessing,
      state.backendConnected,
      showShortcuts,
    ],
  );

  // Render with memoized components for better performance
  if (state.appMode === "setup") {
    // Setup mode - directly render the model selector without any container
    return modelSelectorComponent;
  }
  
  // Chat mode - header, chat interface, status bar
  return (
    <Box flexDirection="column" width="100%" height="100%">
      {/* Header - no margin to avoid double borders */}
      <Box 
        paddingX={theme.styles.box.header.paddingX}
        paddingY={theme.styles.box.header.paddingY}
      >
        <Text {...theme.styles.text.heading}>
          oli • {state.models[state.selectedModel]?.name || "AI Assistant"}
        </Text>
      </Box>

      {/* Chat interface - flex grow to fill available space */}
      <Box flexGrow={1} flexDirection="column">
        {chatInterfaceComponent}
      </Box>

      {/* Status bar - fixed at bottom */}
      {statusBarComponent}
    </Box>
  );
};

export default App;
