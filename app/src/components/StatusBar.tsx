import React, { useMemo } from "react";
import { Box, Text } from "ink";
import theme from "../styles/gruvbox.js";
import AnimatedSpinner from "./AnimatedSpinner.js";

// Component props
interface StatusBarProps {
  modelName: string;
  isProcessing: boolean;
  backendConnected?: boolean;
  showShortcuts?: boolean;
}

// Status bar component - modern minimalist design
const StatusBar: React.FC<StatusBarProps> = ({
  modelName,
  isProcessing,
  backendConnected = false,
  showShortcuts = false,
}) => {
  // Get connection status icon and color - memoized to prevent rerenders
  const status = useMemo(() => {
    if (isProcessing) {
      return {
        icon: <AnimatedSpinner color={theme.styles.status.processing.color} />,
        color: theme.styles.status.processing.color,
        text: "Processing",
      };
    } else if (backendConnected) {
      return {
        icon: theme.styles.status.active.icon,
        color: theme.styles.status.active.color,
        text: "Ready",
      };
    } else {
      return {
        icon: theme.styles.status.error.icon,
        color: theme.styles.status.error.color,
        text: "Disconnected",
      };
    }
  }, [isProcessing, backendConnected]);

  return (
    <Box
      paddingX={2}
      paddingY={1}
      flexDirection="row"
      alignItems="center"
      width="100%"
    >
      {/* Status and model info aligned to the left */}
      <Box flexDirection="row" alignItems="center">
        {/* Status indicator */}
        <Box marginRight={2}>
          <Text>
            <Text color={status.color}>{status.icon}</Text>
            <Text {...theme.styles.text.dimmed}> {status.text}</Text>
          </Text>
        </Box>

        {/* Model info */}
        <Box marginRight={2} flexDirection="row" alignItems="center">
          <Text {...theme.styles.text.dimmed}>Model:</Text>
          <Text {...theme.styles.text.highlight}> {modelName}</Text>
        </Box>

        {/* Separator */}
        <Box marginRight={2}>
          <Text {...theme.styles.text.dimmed}>|</Text>
        </Box>

        {/* Shortcuts */}
        <Box flexDirection="row" alignItems="center">
          <Text
            {...theme.styles.text.dimmed}
            color={
              showShortcuts ? theme.colors.dark.yellow : theme.colors.dark.gray
            }
            bold={showShortcuts}
          >
            ? shortcuts
          </Text>
          <Text {...theme.styles.text.dimmed}> | </Text>
          <Text {...theme.styles.text.dimmed}>Ctrl+C to exit</Text>
        </Box>
      </Box>

      {/* Empty box to maintain full width */}
      <Box flexGrow={1} />
    </Box>
  );
};

export default React.memo(StatusBar);
