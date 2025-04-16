import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';

// Task interface
interface Task {
  id: string;
  description: string;
  status: 'in_progress' | 'completed' | 'failed';
  tool_count: number;
  input_tokens: number;
  output_tokens: number;
  created_at: number;
}

// Component props
interface TaskListProps {
  tasks: Task[];
}

// Format elapsed time
const formatElapsedTime = (seconds: number): string => {
  if (seconds < 60) {
    return `${seconds}s`;
  } else if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  }
};

// Task list component
const TaskList: React.FC<TaskListProps> = ({ tasks }) => {
  // No tasks
  if (tasks.length === 0) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold underline>Tasks</Text>
        <Box marginTop={1}>
          <Text dimColor>No tasks yet</Text>
        </Box>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" padding={1}>
      <Text bold underline>Tasks</Text>
      
      {/* Task list */}
      <Box flexDirection="column" marginTop={1}>
        {tasks.map(task => (
          <Box key={task.id} flexDirection="column" marginBottom={1}>
            {/* Task status indicator and description */}
            <Box>
              {task.status === 'in_progress' ? (
                <Text color="yellow">
                  <Spinner type="dots" /> 
                </Text>
              ) : task.status === 'completed' ? (
                <Text color="green">✓ </Text>
              ) : (
                <Text color="red">✗ </Text>
              )}
              <Text>{task.description}</Text>
            </Box>
            
            {/* Task details */}
            <Box marginLeft={2} flexDirection="column">
              <Text dimColor>
                Tools: {task.tool_count} | Time: {formatElapsedTime(task.created_at)}
              </Text>
              {task.status === 'completed' && (
                <Text dimColor>
                  Tokens: {task.input_tokens + task.output_tokens}
                </Text>
              )}
            </Box>
          </Box>
        ))}
      </Box>
    </Box>
  );
};

export default TaskList;