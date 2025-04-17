import { Message, MessageRole } from "../types/index.js";

/**
 * Creates a unique ID for a message
 * @param role Message role
 * @returns Unique message ID
 */
export const createMessageId = (role: MessageRole): string => {
  return `${role}-${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
};

/**
 * Creates a new message object
 * @param role Message role
 * @param content Message content
 * @param taskId Optional task ID
 * @param tool Optional tool name
 * @returns New message object
 */
export const createMessage = (
  role: MessageRole,
  content: string,
  taskId?: string,
  tool?: string,
): Message => {
  return {
    id: createMessageId(role),
    role,
    content,
    timestamp: Date.now(),
    ...(taskId && { task_id: taskId }),
    ...(tool && { tool }),
  };
};

/**
 * Creates multiple messages at once
 * @param messages Array of role and content pairs
 * @returns Array of message objects
 */
export const createMessages = (
  messages: Array<{ role: MessageRole; content: string }>,
): Message[] => {
  return messages.map((msg) => createMessage(msg.role, msg.content));
};
