import { AppState } from "../types/index.js";
import { BackendService } from "../services/backend.js";
import { createMessages } from "./messageUtils.js";
import { getHelpMessage } from "./commandUtils.js";

/**
 * Function type for command handlers
 */
type CommandHandler = (
  command: string,
  state: AppState,
  setState: React.Dispatch<React.SetStateAction<AppState>>,
  backend: BackendService,
  additionalHandlers?: {
    handleClearHistory?: () => void;
    handleModelSelect?: (index: number) => void;
  },
) => void;

/**
 * Handle help command
 */
export const handleHelpCommand: CommandHandler = (command, state, setState) => {
  // Get version from state (populated during backend connection)
  const version = state.backendInfo?.version as string;

  // If version is not available, throw an error
  if (!version) {
    const errorMessage =
      "Error: Unable to fetch version information from backend";
    const errorSystemMessage = createMessages([
      { role: "system", content: errorMessage },
    ]);

    setState((prev) => ({
      ...prev,
      messages: [...prev.messages, ...errorSystemMessage],
    }));
    return;
  }

  // Generate help message with version
  const helpMessage = getHelpMessage(version);

  // Add user command and help message to chat
  const messages = createMessages([
    { role: "user", content: command },
    { role: "system", content: helpMessage },
  ]);

  setState((prev) => ({
    ...prev,
    messages: [...prev.messages, ...messages],
  }));
};

/**
 * Handle clear command
 */
export const handleClearCommand: CommandHandler = async (
  command,
  state,
  setState,
  backend,
  additionalHandlers,
) => {
  // First add user command to chat
  const messages = createMessages([{ role: "user", content: command }]);

  setState((prev) => ({
    ...prev,
    messages: [...prev.messages, messages[0]],
  }));

  // Set loading state
  setState((prev) => ({
    ...prev,
    isProcessing: true,
  }));

  try {
    // Call the backend to clear conversation history
    await backend.call("clear_conversation", {});

    // Use the local clear history handler to clear UI messages
    if (additionalHandlers?.handleClearHistory) {
      additionalHandlers.handleClearHistory();
    }

    // Add success message
    const successMessage = createMessages([
      {
        role: "system",
        content: "Conversation history cleared successfully.",
      },
    ]);

    setState((prev) => ({
      ...prev,
      messages: [...prev.messages, ...successMessage],
      isProcessing: false,
    }));
  } catch (error) {
    // Handle any errors from backend
    const errorMessage = error instanceof Error ? error.message : String(error);

    // Create error message
    const systemMessage = createMessages([
      {
        role: "system",
        content: `Error clearing conversation: ${errorMessage}`,
      },
    ])[0];

    // Add error message
    setState((prev) => ({
      ...prev,
      messages: [...prev.messages, systemMessage],
      isProcessing: false,
      error: errorMessage,
    }));
  }
};

/**
 * Handle exit command
 */
export const handleExitCommand: CommandHandler = () => {
  process.exit(0);
};

/**
 * Handle model command
 */
export const handleModelCommand: CommandHandler = (
  command,
  state,
  setState,
) => {
  // Add user command to chat first
  const messages = createMessages([{ role: "user", content: command }]);

  // Just switch to setup mode without additional message
  setState((prev) => ({
    ...prev,
    messages: [...prev.messages, messages[0]],
    appMode: "setup", // Switch to setup mode
  }));
};

/**
 * Command handler mapping
 */
export const commandHandlers: Record<string, CommandHandler> = {
  "/help": handleHelpCommand,
  "/clear": handleClearCommand,
  "/exit": handleExitCommand,
  "/model": handleModelCommand,
};

/**
 * Execute a command
 * @returns True if the command was handled, false otherwise
 */
export const executeCommand = (
  command: string,
  state: AppState,
  setState: React.Dispatch<React.SetStateAction<AppState>>,
  backend: BackendService,
  additionalHandlers?: {
    handleClearHistory?: () => void;
    handleModelSelect?: (index: number) => void;
  },
): boolean => {
  // Extract base command (e.g., "/model" from "/model 1")
  const baseCommand = command.split(" ")[0];

  // Check if we have a handler for this command
  const handler = commandHandlers[baseCommand];

  if (handler) {
    // Execute the command handler
    handler(command, state, setState, backend, additionalHandlers);
    return true;
  }

  return false;
};

/**
 * Handle a non-command message
 */
export const processUserMessage = async (
  input: string,
  state: AppState,
  setState: React.Dispatch<React.SetStateAction<AppState>>,
  backend: BackendService,
) => {
  // Generate stable message ID
  const userMessage = createMessages([{ role: "user", content: input }])[0];

  // Set processing state
  setState((prev) => ({
    ...prev,
    isProcessing: true,
  }));

  // Add user message
  setState((prev) => ({
    ...prev,
    messages: [...prev.messages, userMessage],
  }));

  try {
    // Send the query to the backend
    const result = await backend.call("query_model", {
      prompt: input,
      model_index: state.selectedModel,
      use_agent: state.useAgent,
    });

    // Create assistant response
    const assistantMessage = createMessages([
      {
        role: "assistant",
        content: result.response as string,
      },
    ])[0];

    // Add assistant response
    setState((prev) => ({
      ...prev,
      messages: [...prev.messages, assistantMessage],
      isProcessing: false,
    }));
  } catch (err) {
    // Handle error
    const errorMessage = err instanceof Error ? err.message : String(err);

    // Create error message
    const systemMessage = createMessages([
      {
        role: "system",
        content: `Error: ${errorMessage}`,
      },
    ])[0];

    // Add error message
    setState((prev) => ({
      ...prev,
      messages: [...prev.messages, systemMessage],
      isProcessing: false,
      error: errorMessage,
    }));
  }
};
