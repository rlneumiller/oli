import React, { useMemo } from "react";
import { Box, Text } from "ink";
import theme from "../styles/gruvbox.js";
import { ToolStatus, ToolData } from "../types/index.js";
import AnimatedSpinner from "./AnimatedSpinner.js";

interface ToolStatusIndicatorProps {
  status: ToolStatus;
  data?: ToolData;
  compact?: boolean;
}

const ToolStatusIndicator: React.FC<ToolStatusIndicatorProps> = ({
  status,
  data,
  compact = false,
}) => {
  if (!data) {
    return null;
  }

  // Get the appropriate status indicator and color - memoized to prevent rerenders
  const statusIndicator = useMemo(() => {
    switch (status) {
      case "running":
        return <AnimatedSpinner color={theme.colors.dark.blue} />;
      case "success":
        return <Text color={theme.colors.dark.green}>✓</Text>;
      case "error":
        return <Text color={theme.colors.dark.red}>✗</Text>;
      default:
        return <Text color={theme.colors.dark.gray}>⏺</Text>;
    }
  }, [status]);

  // Format tool name and file path - memoized to prevent recalculation
  const toolTitle = useMemo(() => {
    let title = data.name || "Unknown Tool";

    // Extract file path from data or metadata
    const filePath = data.file_path
      ? data.file_path
      : (data as unknown as { metadata?: { file_path?: string } }).metadata
          ?.file_path;

    if (filePath) {
      // Shorten file path if it's too long
      const displayPath =
        filePath.length > 30
          ? `...${filePath.substring(filePath.length - 30)}`
          : filePath;

      title += ` (${displayPath})`;
    }

    // Add ellipsis if running
    if (status === "running") {
      title += "…";
    }

    return title;
  }, [data.name, data.file_path, status]);

  // Format details based on tool type - memoized to prevent recalculation
  const details = useMemo(() => {
    if (compact) return null;

    // Safely access lines - check both direct property and metadata
    const lines = data.lines
      ? data.lines
      : (data as unknown as { metadata?: { lines?: number } }).metadata?.lines;

    if (lines) {
      return `Read ${lines} lines`;
    }

    // Safely access description - check both direct property and metadata
    const description = data.description
      ? data.description
      : (data as unknown as { message?: string }).message ||
        (data as unknown as { metadata?: { description?: string } }).metadata
          ?.description;

    if (description) {
      return description;
    }

    return null;
  }, [data, compact]);

  // Get appropriate color for status
  const statusColor = useMemo(() => {
    switch (status) {
      case "running":
        return theme.colors.dark.blue;
      case "success":
        return theme.colors.dark.green;
      case "error":
        return theme.colors.dark.red;
      default:
        return theme.colors.dark.gray;
    }
  }, [status]);

  return (
    <Box flexDirection="column">
      <Box flexDirection="row">
        {statusIndicator}
        <Text color={statusColor} bold>
          {" "}
          {toolTitle}
        </Text>
      </Box>

      {details && (
        <Box marginLeft={2} flexDirection="row">
          <Text color={theme.colors.dark.gray}> ⎿ {details}</Text>
        </Box>
      )}
    </Box>
  );
};

// Export the component with React.memo
const MemoizedToolStatusIndicator = React.memo(ToolStatusIndicator);
export { MemoizedToolStatusIndicator as ToolStatusIndicator };
export default MemoizedToolStatusIndicator;
