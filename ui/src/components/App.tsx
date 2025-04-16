import React, { useEffect, useState } from 'react';
import { Box, Text } from 'ink';
import { BackendService } from '../services/backend.js';
import ChatInterface from './ChatInterface.js';
import ModelSelector from './ModelSelector.js';
import StatusBar from './StatusBar.js';
import theme from '../styles/gruvbox.js';

// App props interface
interface AppProps {
  backend: BackendService;
}

// App state interface
interface AppState {
  models: any[];
  selectedModel: number;
  messages: Message[];
  tasks: any[];
  isProcessing: boolean;
  error: string | null;
  backendConnected: boolean;
  appMode: 'setup' | 'chat';  // Add a mode to switch between setup and chat
}

// Message interface
interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  timestamp: number;
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
    appMode: 'setup'  // Start in setup mode
  });

  // Load initial data
  useEffect(() => {
    // Listen for backend connection events
    backend.on('backend_connected', (params) => {
      setState(prev => ({
        ...prev,
        models: params.models || [],
        backendConnected: true,
        messages: [...prev.messages, {
          id: `system-${Date.now()}`,
          role: 'system',
          content: 'Connected to backend successfully',
          timestamp: Date.now()
        }]
      }));
    });
    
    backend.on('backend_connection_error', (params) => {
      setState(prev => ({
        ...prev,
        error: params.error,
        backendConnected: false,
        messages: [...prev.messages, {
          id: `system-${Date.now()}`,
          role: 'system',
          content: `Failed to connect to backend: ${params.error}`,
          timestamp: Date.now()
        }]
      }));
    });

    // Register event listeners for backend notifications
    backend.on('processing_started', (params) => {
      setState(prev => ({
        ...prev,
        isProcessing: true
      }));
    });

    backend.on('processing_progress', (params) => {
      // Add progress message if it's not already in the list
      setState(prev => {
        // Only add the message if it's not a duplicate
        if (!prev.messages.some(m => m.content === params.message)) {
          return {
            ...prev,
            messages: [...prev.messages, {
              id: `progress-${Date.now()}`,
              role: 'system',
              content: params.message,
              timestamp: Date.now()
            }]
          };
        }
        return prev;
      });
    });

    backend.on('processing_complete', (params) => {
      setState(prev => ({
        ...prev,
        isProcessing: false
      }));
    });

    backend.on('tool_execution', (params) => {
      setState(prev => ({
        ...prev,
        messages: [...prev.messages, {
          id: `tool-${Date.now()}`,
          role: 'tool',
          content: `[${params.tool}] ${params.message}`,
          timestamp: Date.now()
        }]
      }));
    });

    backend.on('log_message', (params) => {
      // Silent log handling
    });

    // Clean up event listeners on component unmount
    return () => {
      backend.removeAllListeners();
    };
  }, [backend]);

  // Handle user input
  const handleUserInput = async (input: string) => {
    // Add user message to message list
    const userMessage: Message = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: input,
      timestamp: Date.now()
    };

    setState(prev => ({
      ...prev,
      messages: [...prev.messages, userMessage],
      isProcessing: true
    }));

    try {
      // Send the query to the backend
      const result = await backend.call('query_model', {
        prompt: input,
        model_index: state.selectedModel
      });

      // Add assistant response to message list
      const assistantMessage: Message = {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        content: result.response,
        timestamp: Date.now()
      };

      setState(prev => ({
        ...prev,
        messages: [...prev.messages, assistantMessage],
        isProcessing: false
      }));
    } catch (err: any) {
      setState(prev => ({
        ...prev,
        isProcessing: false,
        error: `Error: ${err.message}`
      }));
    }
  };

  // Handle model selection
  const handleModelSelect = (index: number) => {
    setState(prev => ({
      ...prev,
      selectedModel: index
    }));
  };
  
  // Handle model confirmation and switch to chat mode
  const handleModelConfirm = () => {
    // Only proceed if we have models and backend is connected
    if (state.models.length > 0 && state.backendConnected) {
      setState(prev => ({
        ...prev,
        appMode: 'chat',
        messages: [
          ...prev.messages,
          {
            id: `system-${Date.now()}`,
            role: 'system',
            content: `Using model: ${state.models[state.selectedModel]?.name}. Type a message to begin.`,
            timestamp: Date.now()
          }
        ]
      }));
    }
  };

  // Simple UI with no unnecessary attributes that could cause rendering issues
  return (
    <>
      {state.appMode === 'setup' ? (
        <ModelSelector 
          models={state.models} 
          selectedIndex={state.selectedModel} 
          onSelect={handleModelSelect}
          onConfirm={handleModelConfirm}
          isLoading={!state.backendConnected || state.models.length === 0}
        />
      ) : (
        <Box flexDirection="column">
          {/* Header */}
          <Box 
            borderStyle={theme.styles.box.header.borderStyle}
            borderColor={theme.colors.dark.green}
            paddingX={theme.styles.box.header.paddingX}
            paddingY={theme.styles.box.header.paddingY}
            marginBottom={1}
          >
            <Text {...theme.styles.text.heading}>
              oli • {state.models[state.selectedModel]?.name || 'AI Assistant'}
            </Text>
          </Box>

          {/* Chat interface */}
          <Box flexGrow={1} flexDirection="column">
            <ChatInterface 
              messages={state.messages} 
              isProcessing={state.isProcessing}
              onSubmit={handleUserInput}
            />
          </Box>

          {/* Status bar */}
          <StatusBar 
            modelName={state.models[state.selectedModel]?.name || 'AI Assistant'}
            isProcessing={state.isProcessing}
            backendConnected={state.backendConnected}
          />
        </Box>
      )}
    </>
  );
};

export default App;
