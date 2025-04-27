import { Command } from "../types/index.js";

/**
 * Default commands available in the application
 */
export const AVAILABLE_COMMANDS: Command[] = [
  {
    name: "help",
    description: "Show help and available commands",
    value: "/help",
  },
  {
    name: "clear",
    description: "Clear conversation history and free up context",
    value: "/clear",
  },
  {
    name: "model",
    description: "Switch to model selection mode",
    value: "/model",
  },
  { name: "exit", description: "Exit the application", value: "/exit" },
];

/**
 * Get help message with application information
 * @param version Application version
 * @returns Formatted help message
 */
export const getHelpMessage = (version: string): string => {
  const shortcuts = "  • / - Run a command\n  • Ctrl+J - Insert a new line";

  const commands = AVAILABLE_COMMANDS.map(
    (cmd) => `  • ${cmd.value} - ${cmd.description}`,
  ).join("\n");

  return `⏺ oli Terminal Assistant v${version}

  oli is a terminal-based AI assistant.

  Common Tasks

  • Code explanations: How does this function work?
  • File operations: Search for code, edit files
  • Run commands: Execute shell commands
  • Task execution: Agent-based automated tasks

  Keyboard Shortcuts

${shortcuts}

  Commands

${commands}`;
};

/**
 * Check if a string is a command
 * @param text Text to check
 * @returns True if text is a command
 */
export const isCommand = (text: string): boolean => {
  return text.startsWith("/");
};

/**
 * Get command from text
 * @param text Text containing command
 * @returns Command object or undefined if not found
 */
export const getCommand = (text: string): Command | undefined => {
  // For commands with arguments like "/model 1", just check the base command
  const baseCommand = text.split(" ")[0];
  return AVAILABLE_COMMANDS.find((cmd) => cmd.value === baseCommand);
};
