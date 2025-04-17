import React, { useState, useEffect, useMemo } from "react";
import { Box, Text, useInput } from "ink";
import theme from "../styles/gruvbox.js";

// Command interface
interface Command {
  name: string;
  description: string;
  value: string;
}

// Component props
interface CommandPaletteProps {
  visible: boolean;
  filterText: string;
  onSelect: (command: string) => void;
  onFilteredCommandsChange?: (commands: Command[]) => void;
  onSelectedIndexChange?: (index: number) => void;
}

// Available commands
const AVAILABLE_COMMANDS: Command[] = [
  { name: "help", description: "Show help information", value: "/help" },
  { name: "clear", description: "Clear conversation history", value: "/clear" },
  { name: "model", description: "Change the current model", value: "/model" },
  { name: "exit", description: "Exit the application", value: "/exit" },
];

// Command palette component
const CommandPalette: React.FC<CommandPaletteProps> = ({
  visible,
  filterText,
  onSelect,
  onFilteredCommandsChange,
  onSelectedIndexChange,
}) => {
  const [filteredCommands, setFilteredCommands] = useState<Command[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Update parent component when filteredCommands change
  useEffect(() => {
    if (onFilteredCommandsChange) {
      onFilteredCommandsChange(filteredCommands);
    }
  }, [filteredCommands, onFilteredCommandsChange]);

  // Update parent component when selectedIndex changes
  useEffect(() => {
    if (onSelectedIndexChange) {
      onSelectedIndexChange(selectedIndex);
    }
  }, [selectedIndex, onSelectedIndexChange]);

  // Filter commands based on input with debouncing
  useEffect(() => {
    if (!visible) return;

    // Use timeout to debounce filtering for better performance
    const debounceTimeout = setTimeout(() => {
      // Strip the leading slash for filtering
      const searchText = filterText.startsWith("/")
        ? filterText.slice(1)
        : filterText;

      // Filter commands that match the input
      const filtered = AVAILABLE_COMMANDS.filter((cmd) =>
        cmd.name.toLowerCase().includes(searchText.toLowerCase()),
      );

      setFilteredCommands(filtered);
      // Reset selection to first item when filter changes
      setSelectedIndex(0);
    }, 5); // Small debounce time for better responsiveness

    return () => clearTimeout(debounceTimeout);
  }, [filterText, visible]);

  // Handle keyboard navigation using Ink's useInput hook instead of DOM events
  useInput((input, key) => {
    if (!visible) return;

    if (key.downArrow) {
      setSelectedIndex((prev) =>
        Math.min(prev + 1, filteredCommands.length - 1),
      );
    } else if (key.upArrow) {
      setSelectedIndex((prev) => Math.max(prev - 1, 0));
    } else if (key.return || key.tab) {
      if (filteredCommands.length > 0) {
        onSelect(filteredCommands[selectedIndex].value);
      }
    }
  });

  // Enhanced descriptions for commands
  const enhancedCommands = useMemo(() => {
    return [
      {
        name: "help",
        description: "Show help and available commands",
        value: "/help",
      },
      {
        name: "clear",
        description: "Clear conversation history and free up context",
        value: "/clear",
      },
      {
        name: "model",
        description: "Change the current model",
        value: "/model",
      },
      { name: "exit", description: "Exit the application", value: "/exit" },
    ].filter((cmd) => filteredCommands.some((fc) => fc.name === cmd.name));
  }, [filteredCommands]);

  // Memoize the command list for efficient rendering
  const commandItems = useMemo(() => {
    return enhancedCommands.map((command, index) => {
      const isSelected = index === selectedIndex;
      return (
        <Box key={command.value} paddingY={0} paddingX={2} marginY={0}>
          <Box width={16} marginRight={2}>
            <Text
              bold
              color={
                isSelected ? theme.colors.dark.yellow : theme.colors.dark.green
              }
            >
              /{command.name}
            </Text>
          </Box>
          <Text
            color={
              isSelected ? theme.colors.dark.yellow : theme.colors.dark.fg4
            }
          >
            {command.description}
          </Text>
        </Box>
      );
    });
  }, [enhancedCommands, selectedIndex]);

  if (!visible || filteredCommands.length === 0) return null;

  return (
    <Box flexDirection="column" width="100%" marginBottom={1}>
      {commandItems}
    </Box>
  );
};

export default CommandPalette;
