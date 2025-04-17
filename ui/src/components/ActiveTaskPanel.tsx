import React, { useState, useEffect, useMemo } from "react";
import { Box, Text, useInput } from "ink";
import AnimatedSpinner from "./AnimatedSpinner.js";
import theme from "../styles/gruvbox.js";
import { Message } from "../types/index.js";

interface ActiveTaskPanelProps {
  isProcessing: boolean;
  toolMessages: Message[];
  onInterrupt: () => void;
}

// Formats elapsed seconds nicely as MM:SS
const formatElapsedTime = (seconds: number): string => {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
};

const ActiveTaskPanel: React.FC<ActiveTaskPanelProps> = ({
  isProcessing,
  toolMessages,
  onInterrupt,
}) => {
  const [elapsedTime, setElapsedTime] = useState(0);
  const [showPanel, setShowPanel] = useState(false);

  // Count active tools
  const activeToolCount = useMemo(() => {
    return toolMessages.filter(
      (msg) => msg.role === "tool" && msg.tool_status === "running",
    ).length;
  }, [toolMessages]);

  // Set up elapsed time counter
  useEffect(() => {
    let timerId: NodeJS.Timeout | null = null;

    if (isProcessing) {
      setShowPanel(true);
      setElapsedTime(0);

      // Update elapsed time every second
      timerId = setInterval(() => {
        setElapsedTime((prev) => prev + 1);
      }, 1000);
    } else {
      // Keep panel visible for 2 seconds after processing completes
      if (showPanel) {
        setTimeout(() => {
          setShowPanel(false);
        }, 2000);
      }
    }

    return () => {
      if (timerId) clearInterval(timerId);
    };
  }, [isProcessing]);

  // Handle ESC key for interruption
  useInput((input, key) => {
    if (key.escape && isProcessing) {
      onInterrupt();
    }
  });

  // Don't render anything if not processing and panel not visible
  if (!showPanel) return null;

  // We don't need to find the current task anymore

  // Show active task panel
  return (
    <Box flexDirection="column" paddingX={1} marginBottom={1}>
      <Box flexDirection="row" alignItems="center">
        <Box width={2} />
        {/* Add padding before spinner to align with input box */}
        <AnimatedSpinner color={theme.colors.dark.yellow} />
        <Box marginLeft={1} />
        <Text color={theme.colors.dark.gray}>
          {formatElapsedTime(elapsedTime)}
        </Text>
        <Text color={theme.colors.dark.gray}> • </Text>
        <Text color={theme.colors.dark.gray}>{activeToolCount} tools used</Text>
        <Text color={theme.colors.dark.gray}> • </Text>
        <Text color={theme.colors.dark.red}>Press ESC to interrupt</Text>
      </Box>
    </Box>
  );
};

export default React.memo(ActiveTaskPanel);
