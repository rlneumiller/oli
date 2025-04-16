import React, { useState, useEffect } from "react";
import { Box, Text } from "ink";
import TextInput from "ink-text-input";
import Spinner from "ink-spinner";
import theme from "../styles/gruvbox.js";

// Message interface
interface Message {
  id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  timestamp: number;
}

// Component props
interface ChatInterfaceProps {
  messages: Message[];
  isProcessing: boolean;
  onSubmit: (input: string) => void;
}

// Chat interface component
const ChatInterface: React.FC<ChatInterfaceProps> = ({
  messages,
  isProcessing,
  onSubmit,
}) => {
  const [input, setInput] = useState("");
  const [visibleMessages, setVisibleMessages] = useState<Message[]>([]);

  // Update visible messages when messages change
  useEffect(() => {
    // Only show the last 20 messages to prevent terminal overflow
    setVisibleMessages(messages.slice(-20));
  }, [messages]);

  // Handle input submission
  const handleSubmit = (value: string) => {
    if (value.trim() === "") return;
    onSubmit(value);
    setInput("");
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
      <Box marginY={1} paddingX={1}>
        {message.role === "user" ? (
          <Box>
            <Text color={theme.colors.dark.blue} bold>
              {">"}
            </Text>
            <Box marginLeft={1}>
              <Text {...style}>{message.content}</Text>
            </Box>
          </Box>
        ) : message.role === "assistant" ? (
          <Box marginLeft={0}>
            <Text {...style} wrap="wrap">
              {message.content}
            </Text>
          </Box>
        ) : (
          <Box>
            <Text {...style} wrap="wrap">
              {message.content}
            </Text>
          </Box>
        )}
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1} width="100%">
      {/* Messages area - clean and modern */}
      <Box flexDirection="column" flexGrow={1} padding={1} width="100%">
        {visibleMessages.length === 0 ? (
          <Box
            flexGrow={1}
            alignItems="center"
            justifyContent="center"
            flexDirection="column"
            padding={2}
          >
            <Text {...theme.styles.text.highlight}>
              Welcome to Oli AI Assistant
            </Text>
            <Box marginTop={1}>
              <Text {...theme.styles.text.dimmed}>
                Type your message below to start the conversation
              </Text>
            </Box>
          </Box>
        ) : (
          visibleMessages.map((message) => (
            <Box key={message.id}>
              {formatMessage(message)}
              {/* Simple space between messages */}
              {message.role === "assistant" && <Box marginY={1} />}
            </Box>
          ))
        )}
      </Box>

      {/* Simple input area without borders */}
      <Box
        paddingX={2}
        paddingY={1}
        marginTop={1}
        flexDirection="row"
        alignItems="center"
      >
        {isProcessing ? (
          <Box marginRight={1}>
            <Text color={theme.colors.dark.yellow}>
              <Spinner type="dots" />
            </Text>
          </Box>
        ) : (
          <Text color={theme.colors.dark.blue} bold>
            {">"}
          </Text>
        )}

        <Box marginLeft={1} flexGrow={1}>
          <TextInput
            value={input}
            onChange={setInput}
            onSubmit={handleSubmit}
            placeholder={
              isProcessing
                ? "Processing your request..."
                : "Type your message here..."
            }
          />
        </Box>
      </Box>
    </Box>
  );
};

export default ChatInterface;
