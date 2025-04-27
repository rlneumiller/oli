import React from "react";
import { Box, Text } from "ink";
import theme from "../styles/gruvbox.js";

// Simple welcome box component
const WelcomeBox = ({ children }: { children: React.ReactNode }) => (
  <Box
    borderStyle="round"
    borderColor={theme.colors.dark.green}
    paddingX={4}
    paddingY={2}
    width={60}
    flexDirection="column"
    alignSelf="center"
    marginY={2} // Add some margin to center vertically
  >
    <Text color={theme.colors.dark.green} bold>
      ✻ Welcome to oli!
    </Text>
    <Box marginY={1} />
    {children}
    <Box marginY={1} />
    <Text color={theme.colors.dark.fg4}>cwd: {process.cwd()}</Text>
  </Box>
);

export default WelcomeBox;
