import type { AgentRequest } from "./bridge";

export interface AgentSdkContinuation {
  sessionId: string;
  modelKey: string;
  systemPrompt: string;
  toolDescription: string;
  conversationEntries: string[];
}

export interface ContinuationPlan {
  resumeSessionId?: string;
  conversationEntries: string[];
}

/**
 * Reuse an SDK session only when Pi's conversation is an append-only extension
 * of the context that started the previous request. The first appended entry is
 * Pi's persisted copy of the SDK assistant response, which the resumed SDK
 * session already contains, so only entries after it are sent.
 */
export function planContinuation(
  previous: AgentSdkContinuation | undefined,
  request: AgentRequest,
  modelKey: string,
): ContinuationPlan {
  if (
    !previous ||
    previous.modelKey !== modelKey ||
    previous.systemPrompt !== request.systemPrompt ||
    previous.toolDescription !== request.toolDescription ||
    request.conversationEntries.length <= previous.conversationEntries.length
  ) {
    return { conversationEntries: request.conversationEntries };
  }

  const hasPreviousPrefix = previous.conversationEntries.every(
    (entry, index) => request.conversationEntries[index] === entry,
  );
  const mirroredAssistant = request.conversationEntries[previous.conversationEntries.length];
  if (!hasPreviousPrefix || !mirroredAssistant?.startsWith('{"role":"assistant"')) {
    return { conversationEntries: request.conversationEntries };
  }

  return {
    resumeSessionId: previous.sessionId,
    conversationEntries: request.conversationEntries.slice(previous.conversationEntries.length + 1),
  };
}

export function conversationPrompt(entries: readonly string[]): string {
  return [
    "New Pi conversation entries (JSONL):",
    "<pi_conversation>",
    ...entries,
    "</pi_conversation>",
    "Continue from the final conversation entry.",
  ].join("\n");
}
