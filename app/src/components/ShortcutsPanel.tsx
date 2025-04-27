import React from "react";
import { Box, Text } from "ink";
import theme from "../styles/gruvbox.js";

// Component props
interface ShortcutsPanelProps {
  visible: boolean;
}

// Shortcuts panel component
const ShortcutsPanel: React.FC<ShortcutsPanelProps> = ({ visible }) => {
  if (!visible) return null;

  const shortcuts = [
    { key: "/", description: "Run a command" },
    { key: "Ctrl+J", description: "Insert a new line" },
  ];

  return (
    <Box flexDirection="column" padding={1} marginY={1}>
      {shortcuts.map((shortcut) => (
        <Box key={shortcut.key} marginY={0} flexDirection="row">
          <Box width={12}>
            <Text bold color={theme.colors.dark.blue}>
              {shortcut.key}
            </Text>
          </Box>
          <Text>{shortcut.description}</Text>
        </Box>
      ))}
    </Box>
  );
};

export default ShortcutsPanel;
