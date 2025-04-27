import React from "react";
import { useInput } from "ink";
import { BackendService } from "../services/backend.js";

interface TaskInterruptionHandlerProps {
  isProcessing: boolean;
  onInterrupt: () => void;
}

// Component that handles Esc key detection for task interruption
// This is separated to ensure keyboard handlers don't cause re-renders of parent components
const TaskInterruptionHandler: React.FC<TaskInterruptionHandlerProps> = ({
  isProcessing,
  onInterrupt,
}) => {
  // Set up the input handler for detecting Esc key
  useInput((input, key) => {
    if (key.escape && isProcessing) {
      onInterrupt();
    }
  });

  // No visible UI - this is just a keyboard handler
  return null;
};

export default React.memo(TaskInterruptionHandler);

// This utility function creates a handler that can be used to interrupt tasks
export const createInterruptHandler = (
  backend: BackendService,
  setIsProcessing: (value: boolean) => void,
  addSystemMessage: (message: string) => void,
) => {
  return () => {
    // Send interrupt signal to backend
    backend
      .call("interrupt_processing", {})
      .then(() => {
        // Update UI state
        setIsProcessing(false);
        addSystemMessage("Task interrupted by user");
      })
      .catch((error) => {
        console.error("Failed to interrupt processing:", error);
        // Still update UI state in case of failure
        setIsProcessing(false);
        addSystemMessage(
          "Task interruption attempted, but failed to communicate with backend",
        );
      });
  };
};
