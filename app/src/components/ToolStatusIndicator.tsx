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

  // Format tool name and file path or pattern - memoized to prevent recalculation
  const toolTitle = useMemo(() => {
    let title = data.name || "Unknown Tool";

    // Special handling for Search tool (GlobTool)
    if (data.name === "Search") {
      // Try to get pattern directly from metadata
      const pattern = data.metadata?.pattern as string;

      if (pattern) {
        return `Search(pattern: "${pattern}")${status === "running" ? "…" : ""}`;
      }
    }

    // Extract file path from data or metadata (for other tools)
    const filePath =
      data.file_path ?? (data.metadata?.file_path as string | undefined);

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
  }, [data.name, data.file_path, status, data.metadata?.pattern]);

  // Format details based on tool type - memoized to prevent recalculation
  const details = useMemo(() => {
    if (compact) return null;

    // Special handling for Search tool (GlobTool)
    if (data.name === "Search") {
      const count = data.metadata?.count as number;

      if (count !== undefined) {
        return `Found ${count} files`;
      }
    }

    // Safely access lines - check both direct property and metadata
    const lines = data.lines ?? (data.metadata?.lines as number | undefined);

    if (lines) {
      return `Read ${lines} lines`;
    }

    // Safely access description from various possible locations
    const message = data.metadata?.message as string | undefined;
    const metadataDescription = data.metadata?.description as
      | string
      | undefined;
    const description = data.description ?? message ?? metadataDescription;

    if (description) {
      return description;
    }

    return null;
  }, [data.name, data.lines, data.description, data.metadata, compact]);

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
