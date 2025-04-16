import React from "react";
import { Box, Text } from "ink";
import Spinner from "ink-spinner";
import theme from "../styles/gruvbox.js";

// Component props
interface StatusBarProps {
  modelName: string;
  isProcessing: boolean;
  backendConnected?: boolean;
}

// Status bar component - modern minimalist design
const StatusBar: React.FC<StatusBarProps> = ({
  modelName,
  isProcessing,
  backendConnected = false,
}) => {
  // Get connection status icon and color
  const getStatusIndicator = () => {
    if (isProcessing) {
      return {
        icon: <Spinner type="dots" />,
        color: theme.styles.status.processing.color,
      };
    } else if (backendConnected) {
      return {
        icon: theme.styles.status.active.icon,
        color: theme.styles.status.active.color,
      };
    } else {
      return {
        icon: theme.styles.status.error.icon,
        color: theme.styles.status.error.color,
      };
    }
  };

  const status = getStatusIndicator();

  return (
    <Box
      paddingX={2}
      paddingY={0}
      marginTop={1}
      flexDirection="row"
      justifyContent="space-between"
      alignItems="center"
      width="100%"
    >
      {/* Left side: Model info in subtle presentation */}
      <Box flexGrow={1}>
        <Text {...theme.styles.text.dimmed}>Model:</Text>
        <Text {...theme.styles.text.highlight}> {modelName.split(" ")[0]}</Text>
      </Box>

      {/* Middle: Status indicator */}
      <Box>
        <Text>
          <Text color={status.color}>{status.icon}</Text>
          <Text {...theme.styles.text.dimmed}>
            {" "}
            {isProcessing
              ? "Processing"
              : backendConnected
                ? "Ready"
                : "Disconnected"}
          </Text>
        </Text>
      </Box>

      {/* Right side: Exit hint */}
      <Box marginLeft={2}>
        <Text {...theme.styles.text.dimmed}>Ctrl+C to exit</Text>
      </Box>
    </Box>
  );
};

export default StatusBar;
