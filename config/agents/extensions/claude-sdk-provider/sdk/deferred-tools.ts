import { tool } from "@anthropic-ai/claude-agent-sdk";
import type { HookCallback } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
import { InvalidDeferredCallError } from "./errors";
import { record } from "./event-translation";

/** A Pi tool request captured by the SDK hook and deferred to Pi for execution. */
export interface DeferredCall {
  /** SDK tool-use identifier. */
  readonly id: string;
  /** Exact Pi tool name. */
  readonly name: string;
  /** Parsed object arguments supplied for the Pi tool. */
  readonly arguments: Readonly<Record<string, unknown>>;
}

interface DeferredPiCallInput {
  readonly name: string;
  readonly arguments: Record<string, unknown>;
}

const PI_CALL_INPUT_SCHEMA = {
  name: z
    .string()
    .describe('Exact Pi tool name from the available-tools catalog; never "pi_call" itself, which is this gateway\'s own name'),
  arguments: z.record(z.string(), z.unknown()).describe("Arguments matching that Pi tool's input schema"),
};

function sampleToolNames(availableTools: ReadonlySet<string>): string {
  const names = [...availableTools].slice(0, 5);
  return names.length > 0 ? names.join(", ") : "(no Pi tools are available this turn)";
}

function invalidDeferredCallMessage(
  availableTools: ReadonlySet<string>,
  requestedName: string,
  hasArguments: boolean,
): string {
  if (requestedName === "pi_call") {
    return (
      `Invalid Pi tool call: "pi_call" is this gateway's own name, not a Pi tool; do not pass it as ` +
      `the "name" field. Pass the target Pi tool's name instead, e.g. ${sampleToolNames(availableTools)}.`
    );
  }
  if (!hasArguments) {
    return `Invalid Pi tool call: "arguments" must be an object matching "${requestedName || "<missing>"}"'s input schema.`;
  }
  return `Invalid Pi tool call: "${requestedName || "<missing>"}" is not a recognized Pi tool. Available tools: ${sampleToolNames(availableTools)}.`;
}

function parseDeferredCall(
  availableTools: ReadonlySet<string>,
  input: unknown,
):
  | { readonly _tag: "success"; readonly call: Omit<DeferredCall, "id"> }
  | { readonly _tag: "failure"; readonly error: InvalidDeferredCallError } {
  const fields = record(input);
  const requestedName = typeof fields?.name === "string" ? fields.name : "";
  const requestedArguments = record(fields?.arguments);
  if (availableTools.has(requestedName) && requestedArguments) {
    return { _tag: "success", call: { name: requestedName, arguments: requestedArguments } };
  }
  const message = invalidDeferredCallMessage(availableTools, requestedName, requestedArguments !== undefined);
  return { _tag: "failure", error: new InvalidDeferredCallError(requestedName, message) };
}

/**
 * Create the defensive MCP handler used when the SDK fails to honor the defer hook.
 *
 * @returns A handler that always reports a failed, unexecuted tool request.
 */
export function createDeferredPiCallHandler(): (
  input: DeferredPiCallInput,
) => Promise<{ readonly content: Array<{ readonly type: "text"; readonly text: string }>; readonly isError: true }> {
  return async () => ({
    content: [
      {
        type: "text",
        text: "Pi's PreToolUse defer decision was not honored by the Claude Agent SDK; this tool call did not run and was not forwarded to Pi.",
      },
    ],
    isError: true,
  });
}

const DEFERRED_PI_CALL_HANDLER = createDeferredPiCallHandler();

/**
 * Create the in-process MCP gateway exposed to the Claude Agent SDK.
 *
 * @param description - Per-turn Pi tool catalog included in the gateway description.
 * @returns The SDK MCP tool definition.
 */
export function createDeferredPiCallTool(description: string) {
  return tool("pi_call", description, PI_CALL_INPUT_SCHEMA, DEFERRED_PI_CALL_HANDLER);
}

/**
 * Create the hook that parses gateway requests and defers valid calls to Pi.
 *
 * @param availableTools - Pi tool names available during this turn.
 * @param onToolCall - Receives each parsed deferred call.
 * @param onInvalidCall - Receives each rejected call as a typed error.
 * @returns The SDK `PreToolUse` callback.
 */
export function createPreToolUseHook(
  availableTools: ReadonlySet<string>,
  onToolCall: (toolCall: DeferredCall) => void,
  onInvalidCall: (error: InvalidDeferredCallError) => void,
): HookCallback {
  return async (input) => {
    if (input.hook_event_name !== "PreToolUse") return {};
    if (input.tool_name !== "mcp__pi__pi_call") {
      return {
        hookSpecificOutput: {
          hookEventName: "PreToolUse",
          permissionDecision: "deny",
          permissionDecisionReason: `Only the Pi deferred-tool gateway is available, not ${input.tool_name}.`,
        },
      };
    }

    const parsed = parseDeferredCall(availableTools, input.tool_input);
    if (parsed._tag === "failure") {
      onInvalidCall(parsed.error);
      return {
        hookSpecificOutput: {
          hookEventName: "PreToolUse",
          permissionDecision: "deny",
          permissionDecisionReason: parsed.error.message,
        },
      };
    }

    onToolCall({ id: input.tool_use_id, ...parsed.call });
    return { hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "defer" } };
  };
}
