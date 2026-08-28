import type { BridgeEvent } from "../bridge";
import { SdkProtocolError, SdkResultError } from "./errors";

type ParseResult<T> =
  | { readonly _tag: "ok"; readonly value: T }
  | { readonly _tag: "err"; readonly error: SdkProtocolError };

function ok<T>(value: T): ParseResult<T> {
  return { _tag: "ok", value };
}

function err<T>(messageType: string, detail: string): ParseResult<T> {
  return { _tag: "err", error: new SdkProtocolError(messageType, detail) };
}

/**
 * Narrow an unknown SDK value to an object record.
 *
 * @param value - Value received from the SDK boundary.
 * @returns The object record, or `undefined` for primitives and null.
 */
export function record(value: unknown): Record<string, unknown> | undefined {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  // SAFETY: The runtime checks above establish a non-null, non-array object. Reading unknown properties through a record preserves their unknown type.
  return value as Record<string, unknown>;
}

function optionalTokenCount(
  usage: Record<string, unknown>,
  field: string,
  messageType: string,
): ParseResult<number> {
  const value = usage[field];
  if (value === undefined) return ok(0);
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return err(messageType, `${field} must be a finite non-negative number`);
  }
  return ok(value);
}

function usageEvent(usageValue: unknown, messageType: string): ParseResult<BridgeEvent> {
  const usage = record(usageValue);
  if (!usage) return err(messageType, "usage must be an object");

  const input = optionalTokenCount(usage, "input_tokens", messageType);
  if (input._tag === "err") return input;
  const output = optionalTokenCount(usage, "output_tokens", messageType);
  if (output._tag === "err") return output;
  const cacheRead = optionalTokenCount(usage, "cache_read_input_tokens", messageType);
  if (cacheRead._tag === "err") return cacheRead;
  const cacheWrite = optionalTokenCount(usage, "cache_creation_input_tokens", messageType);
  if (cacheWrite._tag === "err") return cacheWrite;

  return ok({
    type: "usage",
    input: input.value,
    output: output.value,
    cacheRead: cacheRead.value,
    cacheWrite: cacheWrite.value,
  });
}

/**
 * Parse one SDK stream message into the provider's event language.
 *
 * @param messageValue - Untrusted value yielded by the Claude Agent SDK.
 * @returns A translated event, no event for irrelevant SDK messages, or a protocol error.
 */
export function translateSdkStreamEvent(messageValue: unknown): ParseResult<BridgeEvent | undefined> {
  const message = record(messageValue);
  if (!message) return err("message", "message must be an object");

  if (message.type === "assistant") {
    const assistantMessage = record(message.message);
    if (!assistantMessage) return err("assistant message", "message must be an object");
    return usageEvent(assistantMessage.usage, "assistant usage");
  }

  if (message.type !== "stream_event") return ok(undefined);
  const event = record(message.event);
  if (!event || typeof event.type !== "string") return err("stream event", "event and event.type are required");

  if (event.type === "content_block_delta") {
    const delta = record(event.delta);
    if (!delta || typeof delta.type !== "string") {
      return err("content_block_delta", "delta and delta.type are required");
    }
    if (delta.type === "text_delta") {
      return typeof delta.text === "string"
        ? ok({ type: "text_delta", text: delta.text })
        : err("text_delta", "text must be a string");
    }
    if (delta.type === "thinking_delta") {
      return typeof delta.thinking === "string"
        ? ok({ type: "thinking_delta", text: delta.thinking })
        : err("thinking_delta", "thinking must be a string");
    }
    return ok(undefined);
  }

  if (event.type === "message_start") {
    const startedMessage = record(event.message);
    if (!startedMessage) return err("message_start", "message must be an object");
    return usageEvent(startedMessage.usage, "message_start usage");
  }
  if (event.type === "message_delta") return usageEvent(event.usage, "message_delta usage");
  return ok(undefined);
}

/** Parsed outcome of a terminal SDK result. */
export type ResultOutcome =
  | {
      readonly _tag: "success";
      readonly stopReason: "stop" | "length";
      readonly terminalReason: string | undefined;
    }
  | { readonly _tag: "failure"; readonly error: SdkResultError }
  | { readonly _tag: "malformed"; readonly error: SdkProtocolError };

/**
 * Parse the terminal result emitted by the Claude Agent SDK.
 *
 * @param message - SDK result record.
 * @returns A successful stop, an SDK-reported failure, or a protocol error.
 */
export function resultOutcome(message: Record<string, unknown>): ResultOutcome {
  if (typeof message.is_error !== "boolean") {
    return { _tag: "malformed", error: new SdkProtocolError("result", "is_error must be a boolean") };
  }
  if (message.stop_reason !== null && message.stop_reason !== undefined && typeof message.stop_reason !== "string") {
    return { _tag: "malformed", error: new SdkProtocolError("result", "stop_reason must be a string or null") };
  }
  const knownStopReasons: ReadonlySet<unknown> = new Set([
    undefined,
    null,
    "end_turn",
    "max_tokens",
    "stop_sequence",
    "tool_use",
  ]);
  if (!knownStopReasons.has(message.stop_reason)) {
    return {
      _tag: "malformed",
      error: new SdkProtocolError("result", `unsupported stop_reason ${String(message.stop_reason)}`),
    };
  }
  if (message.terminal_reason !== undefined && typeof message.terminal_reason !== "string") {
    return { _tag: "malformed", error: new SdkProtocolError("result", "terminal_reason must be a string") };
  }

  const stopReason = message.stop_reason === "max_tokens" ? "length" : "stop";
  const terminalReason = typeof message.terminal_reason === "string" ? message.terminal_reason : undefined;
  const deferUnavailable = terminalReason === "tool_deferred_unavailable";
  if (!message.is_error && !deferUnavailable) {
    return { _tag: "success", stopReason, terminalReason };
  }

  const errors = Array.isArray(message.errors)
    ? message.errors.filter((entry): entry is string => typeof entry === "string")
    : [];
  const resultText = typeof message.result === "string" && message.result.length > 0 ? message.result : undefined;
  const defaultMessage = deferUnavailable
    ? "Claude Agent SDK could not honor the deferred Pi tool call (terminal_reason: tool_deferred_unavailable)"
    : "Claude Agent SDK reported an error result";
  return {
    _tag: "failure",
    error: new SdkResultError(terminalReason, errors.join("; ") || resultText || defaultMessage),
  };
}
