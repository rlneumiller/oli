import React, { useMemo } from "react";
import { Box, Text } from "ink";
import AnimatedSpinner from "./AnimatedSpinner.js";
import { ToolExecution } from "../types/index.js";
import theme from "../styles/gruvbox.js";
import { ToolStatusIndicator } from "./ToolStatusIndicator.js";

interface StatusPanelProps {
  toolExecutions: Map<string, ToolExecution>;
  isProcessing: boolean;
  onInterrupt: () => void;
}

/**
 * StatusPanel is now deprecated. Use StatusDisplay instead.
 * This component is kept for backward compatibility.
 */
const StatusPanel: React.FC<StatusPanelProps> = ({
  toolExecutions,
  isProcessing,
  /* eslint-disable-next-line @typescript-eslint/no-unused-vars */
  onInterrupt,
}) => {
  console.warn("StatusPanel is deprecated. Use StatusDisplay instead.");

  // Get the most recent active tool
  const activeTool = useMemo(() => {
    if (!isProcessing) return null;

    const tools = Array.from(toolExecutions.values());

    // Find tools that are currently running
    const runningTools = tools
      .filter((tool) => tool.status === "running")
      .sort((a, b) => b.startTime - a.startTime);

    // Return the most recent running tool, if any
    return runningTools.length > 0 ? runningTools[0] : null;
  }, [toolExecutions, isProcessing]);

  if (!isProcessing) return null;

  return (
    <Box flexDirection="row" justifyContent="space-between">
      {/* Left side: Tool status, if available */}
      <Box>
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
          />
        )}
      </Box>

      {/* Right side: Interrupt instruction */}
      <Box flexDirection="row" alignItems="center">
        <AnimatedSpinner color={theme.colors.dark.yellow} />
        <Box marginLeft={1} />
        <Text color={theme.colors.dark.red}>ESC to interrupt</Text>
      </Box>
    </Box>
  );
};

export default React.memo(StatusPanel);
