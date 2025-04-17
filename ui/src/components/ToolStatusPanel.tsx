import React, { useMemo, useEffect } from "react";
import { Box, Text } from "ink";
import { ToolStatusIndicator } from "./ToolStatusIndicator.js";
import { ToolExecution } from "../types/index.js";
import theme from "../styles/gruvbox.js";

interface ToolStatusPanelProps {
  toolExecutions: Map<string, ToolExecution>;
}

// Component to display tool status updates
const ToolStatusPanel: React.FC<ToolStatusPanelProps> = ({
  toolExecutions,
}) => {
  // Debug log on mount and update
  useEffect(() => {
    console.log(`ToolStatusPanel updated: toolExecutions.size=${toolExecutions.size}`);
    if (toolExecutions.size > 0) {
      const tools = Array.from(toolExecutions.entries());
      console.log(`First tool: ${tools[0][0]} - ${tools[0][1].name} (${tools[0][1].status})`);
    }
  }, [toolExecutions]);

  // Filter and organize tool executions by status
  const { runningTools, recentTools } = useMemo(() => {
    // Convert map to array for easier filtering
    const tools = Array.from(toolExecutions.values());
    
    // Get running tools (sorted most recent first)
    const running = tools
      .filter(tool => tool.status === "running")
      .sort((a, b) => b.startTime - a.startTime);
    
    // Get recently completed/failed tools
    const recent = tools
      .filter(tool => tool.status !== "running")
      .sort((a, b) => {
        // Sort by end time if available, otherwise by start time
        const aTime = a.endTime || a.startTime;
        const bTime = b.endTime || b.startTime;
        return bTime - aTime;
      })
      // Limit to 3 recent tools to avoid cluttering
      .slice(0, 3);
    
    return {
      runningTools: running,
      recentTools: recent
    };
  }, [toolExecutions]);
  
  // Detailed logging for debugging
  console.log(`ToolStatusPanel render: toolExecutions.size=${toolExecutions.size}, running=${runningTools.length}, recent=${recentTools.length}`);
  
  // Don't render if no active or recent tools
  if (runningTools.length === 0 && recentTools.length === 0) return null;
  
  return (
    <Box flexDirection="column" marginY={1} marginX={1}>
      {/* Header for tool status section */}
      <Box paddingX={1} marginBottom={1}>
        <Text color={theme.colors.dark.green} bold>
          Tool Status:
        </Text>
      </Box>
      
      {/* Running tools first */}
      {runningTools.map((tool) => (
        <Box key={tool.id} paddingLeft={2} marginBottom={1}>
          <ToolStatusIndicator
            status={tool.status}
            data={{
              name: tool.name,
              file_path: tool.metadata.file_path,
              lines: tool.metadata.lines,
              description: tool.message || tool.metadata.description,
            }}
          />
        </Box>
      ))}
      
      {/* Recently completed tools */}
      {recentTools.length > 0 && (
        <>
          <Box paddingX={1} marginBottom={1} marginTop={1}>
            <Text color={theme.colors.dark.yellow} dimColor>
              Recent:
            </Text>
          </Box>
          
          {recentTools.map((tool) => (
            <Box key={tool.id} paddingLeft={2} marginBottom={1}>
              <ToolStatusIndicator
                status={tool.status}
                data={{
                  name: tool.name,
                  file_path: tool.metadata.file_path,
                  lines: tool.metadata.lines,
                  description: tool.message || tool.metadata.description,
                }}
              />
            </Box>
          ))}
        </>
      )}
    </Box>
  );
};

export default React.memo(ToolStatusPanel);