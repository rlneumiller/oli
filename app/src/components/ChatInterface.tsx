/* eslint-disable @typescript-eslint/no-unused-vars */
import React, { useState, useEffect, useCallback } from "react";
import { Box, Text, useInput } from "ink";
import TextInput from "ink-text-input";
import theme from "../styles/gruvbox.js";
import ShortcutsPanel from "./ShortcutsPanel.js";
import CommandPalette from "./CommandPalette.js";
import ToolStatusIndicator from "./ToolStatusIndicator.js";
import StatusDisplay from "./StatusDisplay.js";
import TaskInterruptionHandler from "./TaskInterruptionHandler.js";
import { isCommand } from "../utils/commandUtils.js";

// Import types
import { Message, ToolExecution } from "../types/index.js";

// Messages Display component - memoized to only render when messages change
interface MessagesDisplayProps {
  visibleMessages: Message[];
  formatMessage: (message: Message) => React.ReactNode;
}

const MessagesDisplay: React.FC<MessagesDisplayProps> = React.memo(
  ({ visibleMessages, formatMessage }) => {
    return (
      <Box flexDirection="column" flexGrow={1} padding={1}>
        {visibleMessages.length === 0 ? (
          <Box
            flexGrow={1}
            alignItems="center"
            justifyContent="center"
            flexDirection="column"
            padding={2}
          >
            <Text {...theme.styles.text.highlight}>Ready for input...</Text>
          </Box>
        ) : (
          <Box flexDirection="column" flexGrow={1}>
            {visibleMessages.map((message) => (
              <Box key={message.id} marginY={0}>
                {formatMessage(message)}
              </Box>
            ))}
          </Box>
        )}
      </Box>
    );
  },
);

// Input Area component - memoized to prevent unnecessary renders
interface InputAreaProps {
  input: string;
  setInput: (value: string) => void;
  multilineInput: string;
  setMultilineInput: (value: string) => void;
  commandMode: boolean;
  setCommandMode: (value: boolean) => void;
  showCommandPalette: boolean;
  setShowCommandPalette: (value: boolean) => void;
  commandHistory: string[];
  setCommandHistory: (fn: (prev: string[]) => string[]) => void;
  historyIndex: number;
  setHistoryIndex: (value: number) => void;
  filteredCommands: Array<{ value: string; description: string }>;
  selectedIndex: number;
  showShortcuts: boolean;
  onToggleShortcuts?: () => void;
  onExecuteCommand?: (command: string) => void;
  handleCommandSelect: (command: string) => void;
  handleSubmit: (value: string) => void;
}

const InputArea: React.FC<InputAreaProps> = React.memo(
  ({
    input,
    setInput,
    multilineInput,
    setMultilineInput,
    commandMode,
    setCommandMode,
    showCommandPalette,
    setShowCommandPalette,
    commandHistory,
    setCommandHistory,
    historyIndex,
    setHistoryIndex,
    filteredCommands,
    selectedIndex,
    showShortcuts,
    onToggleShortcuts,
    onExecuteCommand,
    handleCommandSelect,
    handleSubmit,
  }) => {
    // Handle onChange for input field
    const handleInputChange = useCallback(
      (value: string) => {
        // Handle ? key for shortcuts (already handled in useInput)
        if (input === "" && value === "?") {
          // Don't update input here
          return;
        }

        // Hide shortcuts panel when user starts typing
        if (showShortcuts) {
          onToggleShortcuts?.();
        }

        // Update input value normally
        setInput(value);

        // Check for / command mode
        if (input === "" && value === "/") {
          // Enter command mode
          setCommandMode(true);
          setShowCommandPalette(true);
          // Update input with /
          setInput("/");
          return;
        }

        // Show/hide command palette based on command mode
        if (commandMode && value.startsWith("/")) {
          setShowCommandPalette(true);
        } else if (commandMode && !value.startsWith("/")) {
          // Exit command mode if user removes the slash
          setCommandMode(false);
          setShowCommandPalette(false);
        }
      },
      [input, commandMode, showShortcuts, onToggleShortcuts],
    );

    // Handle input submission
    const handleInputSubmit = useCallback(
      (value: string) => {
        if (value.trim() === "") return;

        // If in command mode and command palette is visible,
        // we use selected command from palette instead of input value
        if (commandMode && showCommandPalette && filteredCommands?.length > 0) {
          // Get the selected command from the command palette
          const selectedCommand = filteredCommands[selectedIndex]?.value;

          // Use the selected command instead of partial input
          if (selectedCommand) {
            // Handle command selection from palette (this will execute the command)
            handleCommandSelect(selectedCommand);
            return;
          }
        }

        // Reset command mode
        if (commandMode) {
          setCommandMode(false);
          setShowCommandPalette(false);
        }

        // Handle non-selected commands (typed fully by user)
        if (isCommand(value)) {
          setCommandHistory((prev) => [...prev, value]);
          setHistoryIndex(-1);

          // Let the dedicated command handler process it
          if (onExecuteCommand) {
            onExecuteCommand(value);

            // Clear input and exit early - command was handled externally
            setInput("");
            return;
          }
        }

        // For non-commands or when onExecuteCommand isn't available
        if (multilineInput) {
          // For multiline input, combine with existing content
          const fullInput = multilineInput + value;
          handleSubmit(fullInput);
          setMultilineInput("");
        } else {
          // Regular input flow
          handleSubmit(value);
        }

        // Clear input explicitly - this works with ink-text-input
        setInput("");
      },
      [
        commandMode,
        showCommandPalette,
        filteredCommands,
        selectedIndex,
        multilineInput,
        setMultilineInput,
        setCommandMode,
        setShowCommandPalette,
        setCommandHistory,
        setHistoryIndex,
        setInput,
        handleCommandSelect,
        handleSubmit,
        onExecuteCommand,
      ],
    );

    return (
      <Box paddingX={2} paddingY={1} flexDirection="column">
        <Box
          borderStyle={commandMode ? "single" : undefined}
          borderColor={theme.colors.dark.green}
          paddingX={1}
          paddingY={commandMode ? 1 : 0}
          flexDirection="column"
        >
          <Box flexDirection="column" flexGrow={1}>
            {/* Previous lines with proper indentation - only show prompt on first line */}
            {multilineInput.split("\n").map((line, i) => (
              <Box key={i} flexDirection="row">
                {/* Only show prompt character on the first line if there's actual content */}
                {i === 0 && line.trim().length > 0 && (
                  <Text
                    color={
                      commandMode
                        ? theme.colors.dark.green
                        : theme.colors.dark.blue
                    }
                    bold
                  >
                    {commandMode ? "/" : ">"}
                  </Text>
                )}
                {/* No prompt for empty first line or continuation lines */}
                {(i !== 0 || line.trim().length === 0) && <Box width={1}></Box>}
                <Box marginLeft={1}>
                  <Text>{line}</Text>
                </Box>
              </Box>
            ))}

            {/* Current input row with prompt - only show if no multiline input */}
            <Box flexDirection="row">
              {/* Only show prompt if we don't have multiline input */}
              {multilineInput.length === 0 && (
                <Text
                  color={
                    commandMode
                      ? theme.colors.dark.green
                      : theme.colors.dark.blue
                  }
                  bold
                >
                  {commandMode ? "/" : ">"}
                </Text>
              )}
              {/* Otherwise keep the spacing consistent */}
              {multilineInput.length > 0 && <Box width={1}></Box>}

              <Box marginLeft={1} flexGrow={1}>
                <TextInput
                  value={input}
                  onChange={handleInputChange}
                  onSubmit={handleInputSubmit}
                  placeholder={
                    commandMode
                      ? "Type a command or use arrows to navigate..."
                      : ""
                  }
                />
              </Box>
            </Box>
          </Box>
        </Box>
      </Box>
    );
  },
);

// Component props
interface ChatInterfaceProps {
  messages: Message[];
  isProcessing: boolean;
  onSubmit: (input: string) => void;
  onInterrupt?: () => void;
  showShortcuts?: boolean;
  onToggleShortcuts?: () => void;
  onClearHistory?: () => void;
  onExecuteCommand?: (command: string) => void;
  toolExecutions?: Map<string, ToolExecution>;
}

// Chat interface component
const ChatInterface: React.FC<ChatInterfaceProps> = ({
  messages,
  isProcessing,
  onSubmit,
  onInterrupt,
  showShortcuts = false,
  onToggleShortcuts,
  onClearHistory,
  onExecuteCommand,
  toolExecutions = new Map(),
}) => {
  const [input, setInput] = useState("");
  const [visibleMessages, setVisibleMessages] = useState<Message[]>([]);
  const [commandMode, setCommandMode] = useState(false);
  // These are used in the handleInputSubmit callback and useInput hook
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [multilineInput, setMultilineInput] = useState("");
  const [filteredCommands, setFilteredCommands] = useState<
    Array<{ value: string; description: string }>
  >([]);
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Handle keyboard shortcuts
  useInput((inputChar, key) => {
    // Handle ? key to toggle shortcuts panel when input is empty
    if (
      inputChar === "?" &&
      input === "" &&
      !isProcessing &&
      !commandMode &&
      !multilineInput
    ) {
      // Toggle shortcuts panel
      onToggleShortcuts?.();

      // Don't add ? to input
      setInput("");

      return;
    }

    // Ctrl+J to insert a newline (a more reliable cross-platform shortcut)
    if (key.ctrl && inputChar === "j" && !commandMode) {
      // Hide shortcuts panel when entering multiline mode
      if (showShortcuts) {
        onToggleShortcuts?.();
      }

      if (input) {
        setMultilineInput((prev) => prev + input + "\n");
        setInput("");
      } else {
        setMultilineInput((prev) => prev + "\n");
      }
      return;
    }

    // Handle / key specially when it's the only input - for command mode
    if (inputChar === "/" && input === "" && !isProcessing && !commandMode) {
      // Let the TextInput's onChange handler process this
      // This avoids interfering with cursor positioning
      return;
    }

    // ESC key to exit command mode
    if (key.escape && commandMode) {
      setCommandMode(false);
      setShowCommandPalette(false);
      setInput("");
      return;
    }

    // Ctrl+L to clear history
    if (key.ctrl && inputChar === "l") {
      onClearHistory?.();
      return;
    }

    // Handle command mode navigation
    if (commandMode) {
      // Tab for autocomplete is now handled in CommandPalette

      // Up/Down for command history when not showing command palette
      if (!showCommandPalette) {
        if (key.upArrow && commandHistory.length > 0) {
          const newIndex = Math.min(
            commandHistory.length - 1,
            historyIndex + 1,
          );
          setHistoryIndex(newIndex);
          setInput(commandHistory[commandHistory.length - 1 - newIndex] || "");
        }

        if (key.downArrow && historyIndex > -1) {
          const newIndex = Math.max(-1, historyIndex - 1);
          setHistoryIndex(newIndex);
          setInput(
            newIndex === -1
              ? "/"
              : commandHistory[commandHistory.length - 1 - newIndex] || "",
          );
        }
      }
    }
  });

  // Update visible messages when messages change, with debouncing
  useEffect(() => {
    // Only show the last 20 messages to prevent terminal overflow
    // Use setTimeout to debounce frequent updates
    const timer = setTimeout(() => {
      setVisibleMessages(messages.slice(-20));
    }, 10);

    return () => clearTimeout(timer);
  }, [messages]);

  // Tool messages are now handled directly by the StatusDisplay component

  // Handle command selection from the command palette
  const handleCommandSelect = (command: string) => {
    setCommandMode(false);
    setShowCommandPalette(false);

    // Execute the selected command
    if (onExecuteCommand) {
      onExecuteCommand(command);
    } else {
      // Handle common commands if onExecuteCommand is not provided
      if (command === "/clear") {
        onClearHistory?.();
      } else if (command === "/exit") {
        process.exit(0);
      } else {
        // Pass as a normal query if not a recognized command
        onSubmit(command);
      }
    }

    // Add to command history
    setCommandHistory((prev) => [...prev, command]);
    setHistoryIndex(-1);
    setInput("");
  };

  // Handle input submission
  const handleSubmit = (value: string) => {
    if (value.trim() === "") return;

    // Double-check for commands - all commands should be handled by handleExecuteCommand
    if (isCommand(value)) {
      console.log(
        "WARNING: Command reached handleSubmit - this should be handled by onExecuteCommand",
      );

      // Use fallback command handling
      if (value === "/clear") {
        onClearHistory?.();
        return;
      } else if (value === "/help") {
        onToggleShortcuts?.();
        return;
      } else if (value === "/exit") {
        process.exit(0);
      }
    }

    // For non-commands and unknown commands, send as normal input to backend
    onSubmit(value);

    // Reset both input states
    setInput("");
    setMultilineInput("");
  };

  // Get Gruvbox style for a message based on its role
  const getMessageStyle = (role: string) => {
    switch (role) {
      case "user":
        return theme.styles.text.user;
      case "assistant":
        return theme.styles.text.assistant;
      case "system":
        return theme.styles.text.system;
      case "tool":
        return theme.styles.text.tool;
      default:
        return {};
    }
  };

  // Format message content with role prefix and styling
  const formatMessage = (message: Message) => {
    const style = getMessageStyle(message.role);

    return (
      <Box marginY={message.role === "assistant" ? 1 : 0} paddingX={1} flexDirection="column">
        {message.role === "user" ? (
          <Box flexDirection="row">
            <Text color={theme.colors.dark.blue} bold>
              {">"}
            </Text>
            <Box marginLeft={1} flexGrow={1}>
              <Text {...style} wrap="wrap">
                {message.content}
              </Text>
            </Box>
          </Box>
        ) : message.role === "assistant" ? (
          <Box flexGrow={1}>
            <Text {...style} wrap="wrap">
              {message.content}
            </Text>
          </Box>
        ) : message.role === "tool" &&
          message.tool_status &&
          message.tool_data ? (
          <ToolStatusIndicator
            status={message.tool_status}
            data={message.tool_data}
          />
        ) : (
          <Box flexGrow={1}>
            <Text {...style} wrap="wrap">
              {message.content}
            </Text>
          </Box>
        )}
      </Box>
    );
  };

  // Optimized layout with better spacing and grouping
  return (
    <>
      {/* Messages area */}
      <MessagesDisplay
        visibleMessages={visibleMessages}
        formatMessage={formatMessage}
      />

      {/* Unified status display - only renders while processing is active */}
      <StatusDisplay
        toolExecutions={toolExecutions}
        isProcessing={isProcessing}
        onInterrupt={onInterrupt || (() => {})}
      />

      {/* Invisible handler for interruption */}
      <TaskInterruptionHandler
        isProcessing={isProcessing}
        onInterrupt={onInterrupt || (() => {})}
      />

      {/* Input area */}
      <InputArea
        input={input}
        setInput={setInput}
        multilineInput={multilineInput}
        setMultilineInput={setMultilineInput}
        commandMode={commandMode}
        setCommandMode={setCommandMode}
        showCommandPalette={showCommandPalette}
        setShowCommandPalette={setShowCommandPalette}
        commandHistory={commandHistory}
        setCommandHistory={setCommandHistory}
        historyIndex={historyIndex}
        setHistoryIndex={setHistoryIndex}
        filteredCommands={filteredCommands}
        selectedIndex={selectedIndex}
        showShortcuts={showShortcuts}
        onToggleShortcuts={onToggleShortcuts}
        onExecuteCommand={onExecuteCommand}
        handleCommandSelect={handleCommandSelect}
        handleSubmit={handleSubmit}
      />

      {/* Command palette */}
      <CommandPalette
        visible={showCommandPalette}
        filterText={input}
        onSelect={handleCommandSelect}
        onFilteredCommandsChange={setFilteredCommands}
        onSelectedIndexChange={setSelectedIndex}
      />

      {/* Shortcuts panel */}
      <ShortcutsPanel visible={showShortcuts || false} />
    </>
  );
};

export default ChatInterface;
