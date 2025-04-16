import React, { useState, useEffect } from "react";
import { Box, Text, useInput } from "ink";
import Spinner from "ink-spinner";
import theme from "../styles/gruvbox.js";

// Model interface
interface Model {
  name: string;
  description: string;
  supports_agent: boolean;
}

// Component props
interface ModelSelectorProps {
  models: Model[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onConfirm: () => void;
  isLoading: boolean;
}

// Simple welcome box component
const WelcomeBox = ({ children }: { children: React.ReactNode }) => (
  <Box width="100%" height="100%" alignItems="center" justifyContent="center">
    <Box
      borderStyle="round"
      borderColor={theme.colors.dark.green}
      paddingX={4}
      paddingY={2}
      width={60}
    >
      <Box flexDirection="column">
        <Text color={theme.colors.dark.green} bold>
          ✻ Welcome to oli!
        </Text>
        <Box marginY={1} />
        {children}
        <Box marginY={1} />
        <Text color={theme.colors.dark.fg4}>cwd: {process.cwd()}</Text>
      </Box>
    </Box>
  </Box>
);

// Model selector with minimal UI
const ModelSelector: React.FC<ModelSelectorProps> = ({
  models,
  selectedIndex,
  onSelect,
  onConfirm,
  isLoading,
}) => {
  // Track local selected index
  const [index, setIndex] = useState(selectedIndex);

  // Update parent when selection changes
  useEffect(() => {
    if (index !== selectedIndex) {
      onSelect(index);
    }
  }, [index, onSelect, selectedIndex]);

  // Handle keyboard input for selection and confirmation
  useInput((input, key) => {
    if (isLoading || models.length === 0) return;

    if (key.return) {
      onConfirm();
    } else if (key.upArrow && index > 0) {
      setIndex((prev) => prev - 1);
    } else if (key.downArrow && index < models.length - 1) {
      setIndex((prev) => prev + 1);
    }
  });

  // Loading state
  if (isLoading) {
    return (
      <WelcomeBox>
        <Text color={theme.colors.dark.blue}>
          <Spinner type="dots" /> Connecting to backend...
        </Text>
      </WelcomeBox>
    );
  }

  // Error state - no models
  if (models.length === 0) {
    return (
      <WelcomeBox>
        <Text color={theme.colors.dark.red}>
          No models available. Please check API keys.
        </Text>
      </WelcomeBox>
    );
  }

  // Model selection
  return (
    <WelcomeBox>
      <>
        <Text color={theme.colors.dark.yellow}>Select a model:</Text>

        <Box marginY={1} flexDirection="column">
          {models.map((model, i) => (
            <Text
              key={`model-${i}`}
              color={
                i === index ? theme.colors.dark.green : theme.colors.dark.fg
              }
              bold={i === index}
            >
              {i === index ? "● " : "○ "}
              {model.name}
            </Text>
          ))}
        </Box>

        {models[index]?.description && (
          <Text color={theme.colors.dark.fg4} wrap="wrap" dimColor>
            {models[index].description}
          </Text>
        )}

        <Text color={theme.colors.dark.fg4}>
          Use arrow keys ↑↓ to select, Enter to confirm
        </Text>
      </>
    </WelcomeBox>
  );
};

export default ModelSelector;
