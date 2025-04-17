import React, { useEffect, useState, useMemo } from "react";
import { Box, Text } from "ink";
import { ToolExecution } from "../types/index.js";
import { ToolStatusIndicator } from "./ToolStatusIndicator.js";
import AnimatedSpinner from "./AnimatedSpinner.js";
import theme from "../styles/gruvbox.js";

interface StatusDisplayProps {
  toolExecutions: Map<string, ToolExecution>;
  isProcessing: boolean;
  onInterrupt: () => void;
}

const StatusDisplay: React.FC<StatusDisplayProps> = ({
  toolExecutions,
  isProcessing,
  /* eslint-disable-next-line @typescript-eslint/no-unused-vars */
  onInterrupt,
}) => {
  // State to ensure smooth transitions
  const [isVisible, setIsVisible] = useState(false);
  const [elapsedTime, setElapsedTime] = useState(0);

  // Set up elapsed time counter for processing
  useEffect(() => {
    let timerId: NodeJS.Timeout | null = null;

    if (isProcessing) {
      setElapsedTime(0);
      // Update elapsed time every second
      timerId = setInterval(() => {
        setElapsedTime((prev) => prev + 1);
      }, 1000);
    } else {
      setElapsedTime(0);
    }

    return () => {
      if (timerId) clearInterval(timerId);
    };
  }, [isProcessing]);

  // Memoized active tool to prevent unnecessary rerenders
  const activeTool = useMemo(() => {
    // Only check when processing is happening
    if (!isProcessing) return null;

    // Find the most recently started running tool
    const runningTools = Array.from(toolExecutions.values())
      .filter((tool) => tool.status === "running")
      .sort((a, b) => b.startTime - a.startTime);

    return runningTools.length > 0 ? runningTools[0] : null;
  }, [toolExecutions, isProcessing]);

  // Format elapsed time nicely MM:SS
  const formattedTime = useMemo(() => {
    const mins = Math.floor(elapsedTime / 60);
    const secs = Math.floor(elapsedTime % 60);
    return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
  }, [elapsedTime]);

  // Show/hide with animation effect
  useEffect(() => {
    if (isProcessing) {
      setIsVisible(true);
    } else {
      // Small delay before hiding to prevent flashing during quick operations
      const timer = setTimeout(() => {
        setIsVisible(false);
      }, 300);
      return () => clearTimeout(timer);
    }
  }, [isProcessing]);

  // Don't render anything when not visible
  if (!isVisible) return null;

  return (
    <Box flexDirection="row" marginY={1} marginX={1}>
      {/* Left side: Processing indicator with elapsed time and interrupt option */}
      <Box marginLeft={1} flexDirection="row" alignItems="center">
        <AnimatedSpinner color={theme.colors.dark.yellow} />
        <Box marginLeft={1} marginRight={1}>
          <Text color={theme.colors.dark.yellow}>
            {elapsedTime > 0 && `${formattedTime}`}
          </Text>
        </Box>
        <Text color={theme.colors.dark.red}>ESC to interrupt</Text>

        {/* Add a separator when there's also a tool showing */}
        {activeTool && (
          <Box marginX={2}>
            <Text color={theme.colors.dark.gray}>|</Text>
          </Box>
        )}
      </Box>

      {/* Tool status (if available) now positioned next to the timer */}
      {activeTool && (
        <ToolStatusIndicator
          status={activeTool.status}
          data={{
            name: activeTool.name,
            file_path: activeTool.metadata.file_path as string | undefined,
            lines: activeTool.metadata.lines as number | undefined,
            description:
              activeTool.message ||
              (activeTool.metadata.description as string | undefined),
          }}
          compact={true}
        />
      )}

      {/* Empty space to maintain layout */}
      <Box flexGrow={1} />
    </Box>
  );
};

export default React.memo(StatusDisplay);
