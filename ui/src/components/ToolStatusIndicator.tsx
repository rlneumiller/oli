import React, { useMemo } from "react";
import { Box, Text } from "ink";
import theme from "../styles/gruvbox.js";
import { ToolStatus, ToolData } from "../types/index.js";
import AnimatedSpinner from "./AnimatedSpinner.js";

interface ToolStatusIndicatorProps {
  status: ToolStatus;
  data?: ToolData;
}

const ToolStatusIndicator: React.FC<ToolStatusIndicatorProps> = ({
  status,
  data,
}) => {
  if (!data) {
    console.log("ToolStatusIndicator: Missing data prop");
    return null;
  }

  // Log props for debugging
  console.log(`ToolStatusIndicator: ${data.name}, status=${status}`);

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

    // Add file path if available
    if (data.file_path) {
      // Shorten file path if it's too long
      const path = data.file_path;
      const displayPath =
        path.length > 30 ? `...${path.substring(path.length - 30)}` : path;

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
    if (data.lines) {
      return `Read ${data.lines} lines (ctrl+r to expand)`;
    }

    if (data.description) {
      return data.description;
    }

    return null;
  }, [data.lines, data.description]);

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
