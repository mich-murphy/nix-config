import type { BridgeEvent } from "../bridge";

export function record(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : undefined;
}

function number(value: unknown): number {
  return typeof value === "number" ? value : 0;
}

function usageEvent(usageValue: unknown): BridgeEvent | undefined {
  const usage = record(usageValue);
  if (!usage) return undefined;
  return {
    type: "usage",
    input: number(usage.input_tokens),
    output: number(usage.output_tokens),
    cacheRead: number(usage.cache_read_input_tokens),
    cacheWrite: number(usage.cache_creation_input_tokens),
  };
}

export function translateSdkStreamEvent(messageValue: unknown): BridgeEvent | undefined {
  const message = record(messageValue);
  if (!message) return undefined;

  if (message.type === "assistant") {
    return usageEvent(record(message.message)?.usage);
  }

  if (message.type !== "stream_event") return undefined;
  const event = record(message.event);
  if (!event) return undefined;

  if (event.type === "content_block_delta") {
    const delta = record(event.delta);
    if (delta?.type === "text_delta" && typeof delta.text === "string") {
      return { type: "text_delta", text: delta.text };
    }
    if (delta?.type === "thinking_delta" && typeof delta.thinking === "string") {
      return { type: "thinking_delta", text: delta.thinking };
    }
  }

  if (event.type === "message_start") return usageEvent(record(event.message)?.usage);
  if (event.type === "message_delta") return usageEvent(event.usage);
  return undefined;
}

export interface ResultOutcome {
  isError: boolean;
  stopReason: "stop" | "length";
  errorMessage?: string;
}

export function resultOutcome(message: Record<string, unknown>): ResultOutcome {
  const stopReason = message.stop_reason === "max_tokens" ? "length" : "stop";
  const deferUnavailable = message.terminal_reason === "tool_deferred_unavailable";
  if (message.is_error !== true && !deferUnavailable) return { isError: false, stopReason };

  const errors = Array.isArray(message.errors)
    ? message.errors.filter((entry): entry is string => typeof entry === "string")
    : [];
  const resultText = typeof message.result === "string" && message.result.length > 0 ? message.result : undefined;
  const defaultMessage = deferUnavailable
    ? "Claude Agent SDK could not honor the deferred Pi tool call (terminal_reason: tool_deferred_unavailable)"
    : "Claude Agent SDK reported an error result";
  return {
    isError: true,
    stopReason,
    errorMessage: errors.join("; ") || resultText || defaultMessage,
  };
}
