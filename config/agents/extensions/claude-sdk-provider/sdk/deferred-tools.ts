import { tool } from "@anthropic-ai/claude-agent-sdk";
import type { HookCallback } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
import { record } from "./event-translation";

export interface DeferredCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

interface DeferredPiCallInput {
  name: string;
  arguments: Record<string, unknown>;
}

const PI_CALL_INPUT_SCHEMA = {
  name: z
    .string()
    .describe('Exact Pi tool name from the available-tools catalog — never "pi_call" itself, which is this gateway\'s own name'),
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
      `Invalid Pi tool call: "pi_call" is this gateway's own name, not a Pi tool — do not pass it as ` +
      `the "name" field. Pass the target Pi tool's name instead, e.g. ${sampleToolNames(availableTools)}.`
    );
  }
  if (!hasArguments) {
    return `Invalid Pi tool call: "arguments" must be an object matching "${requestedName || "<missing>"}"'s input schema.`;
  }
  return `Invalid Pi tool call: "${requestedName || "<missing>"}" is not a recognized Pi tool. Available tools: ${sampleToolNames(availableTools)}.`;
}

function validateDeferredCall(
  availableTools: ReadonlySet<string>,
  input: unknown,
): { ok: true; call: Omit<DeferredCall, "id"> } | { ok: false; error: Error } {
  const fields = record(input);
  const requestedName = typeof fields?.name === "string" ? fields.name : "";
  const requestedArguments = record(fields?.arguments);
  if (availableTools.has(requestedName) && requestedArguments) {
    return { ok: true, call: { name: requestedName, arguments: requestedArguments } };
  }
  return {
    ok: false,
    error: new Error(invalidDeferredCallMessage(availableTools, requestedName, requestedArguments !== undefined)),
  };
}

export function createDeferredPiCallHandler(): (
  input: DeferredPiCallInput,
) => Promise<{ content: Array<{ type: "text"; text: string }>; isError: true }> {
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

export function createDeferredPiCallTool(description: string) {
  return tool("pi_call", description, PI_CALL_INPUT_SCHEMA, DEFERRED_PI_CALL_HANDLER);
}

export function createPreToolUseHook(
  availableTools: ReadonlySet<string>,
  onToolCall: (toolCall: DeferredCall) => void,
  onInvalidCall: (error: Error) => void,
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

    const validated = validateDeferredCall(availableTools, input.tool_input);
    if (!validated.ok) {
      onInvalidCall(validated.error);
      return {
        hookSpecificOutput: {
          hookEventName: "PreToolUse",
          permissionDecision: "deny",
          permissionDecisionReason: validated.error.message,
        },
      };
    }

    onToolCall({ id: input.tool_use_id, ...validated.call });
    return { hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "defer" } };
  };
}
