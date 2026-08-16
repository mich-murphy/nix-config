import { createSdkMcpServer, query, tool } from "@anthropic-ai/claude-agent-sdk";
import type { Api, Model, SimpleStreamOptions } from "@earendil-works/pi-ai";
import { randomUUID } from "node:crypto";
import { z } from "zod";
import type { AgentRequest, AgentSdkRun, BridgeEvent } from "./bridge";

export type RunSdkQuery = (params: Parameters<typeof query>[0]) => AsyncIterable<unknown>;

const NON_SUBSCRIPTION_AUTH_VARIABLES = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "CLAUDE_CODE_USE_BEDROCK",
  "CLAUDE_CODE_USE_VERTEX",
  "CLAUDE_CODE_USE_FOUNDRY",
] as const;

export function subscriptionEnvironment(
  source: Record<string, string | undefined> = process.env,
): Record<string, string | undefined> {
  const environment = { ...source };
  for (const name of NON_SUBSCRIPTION_AUTH_VARIABLES) delete environment[name];
  environment.CLAUDE_AGENT_SDK_CLIENT_APP = "pi-coding-agent-provider/0.1.0";
  return environment;
}

function record(value: unknown): Record<string, unknown> | undefined {
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
    const assistantMessage = record(message.message);
    return usageEvent(assistantMessage?.usage);
  }

  if (message.type !== "stream_event") return undefined;
  const event = record(message.event);
  if (!event) return undefined;

  if (event.type === "content_block_delta") {
    const delta = record(event.delta);
    if (delta?.type === "text_delta" && typeof delta.text === "string") {
      return { type: "text_delta", text: delta.text };
    }
  }

  if (event.type === "message_start") {
    return usageEvent(record(event.message)?.usage);
  }
  if (event.type === "message_delta") {
    return usageEvent(event.usage);
  }
  return undefined;
}

function effortFor(reasoning: SimpleStreamOptions["reasoning"]): "low" | "medium" | "high" | "xhigh" | "max" | undefined {
  if (!reasoning) return undefined;
  if (reasoning === "minimal") return "low";
  return reasoning;
}

export function agentSdkTurnOptions(): { maxTurns?: number } {
  // Pi owns the outer tool loop. A one-turn SDK cap can terminate before the
  // deferred gateway call is handed back to Pi.
  return {};
}

interface DeferredPiCallInput {
  name: string;
  arguments: Record<string, unknown>;
}

export function createDeferredPiCallHandler(
  availableTools: ReadonlySet<string>,
  onToolCall: (toolCall: Extract<BridgeEvent, { type: "tool_call" }>) => void,
  onError: (error: Error) => void,
  createId: () => string = randomUUID,
): (input: DeferredPiCallInput) => Promise<{ content: Array<{ type: "text"; text: string }>; isError?: boolean }> {
  return async (input) => {
    const requestedName = typeof input.name === "string" ? input.name : "";
    const requestedArguments = record(input.arguments);
    if (!availableTools.has(requestedName) || !requestedArguments) {
      const error = new Error(`Claude requested an invalid Pi tool call: ${requestedName || "<missing>"}`);
      onError(error);
      return { content: [{ type: "text", text: error.message }], isError: true };
    }

    onToolCall({
      type: "tool_call",
      id: createId(),
      name: requestedName,
      arguments: requestedArguments,
    });
    return { content: [{ type: "text", text: "Tool execution is deferred to Pi." }] };
  };
}

export function createClaudeAgentSdkRunner(runSdkQuery: RunSdkQuery = query): AgentSdkRun {
  return async function* runClaudeAgentSdk(
    request: AgentRequest,
    model: Model<Api>,
    options?: SimpleStreamOptions,
  ): AsyncGenerator<BridgeEvent> {
    const abortController = new AbortController();
    const onAbort = () => abortController.abort();
    options?.signal?.addEventListener("abort", onAbort, { once: true });

    let deferredToolCall: Extract<BridgeEvent, { type: "tool_call" }> | undefined;
  let deferredError: Error | undefined;
  const availableTools = new Set(request.toolNames);
  const deferToolCall = (toolCall: Extract<BridgeEvent, { type: "tool_call" }>) => {
    deferredToolCall ??= toolCall;
  };
  const deferError = (error: Error) => {
    deferredError ??= error;
  };
  const handlePiCall = createDeferredPiCallHandler(availableTools, deferToolCall, deferError);

  const piCall = tool(
    "pi_call",
    request.toolDescription,
    {
      name: z.string().describe("Exact Pi tool name from the available-tools catalog"),
      arguments: z.record(z.string(), z.unknown()).describe("Arguments matching that Pi tool's input schema"),
    },
    handlePiCall,
  );
  const server = createSdkMcpServer({ name: "pi", version: "0.1.0", tools: [piCall], alwaysLoad: true });

  try {
    const sdkQuery = runSdkQuery({
      prompt: request.prompt,
      options: {
        abortController,
        cwd: process.cwd(),
        model: model.id,
        effort: effortFor(options?.reasoning),
        includePartialMessages: true,
        ...agentSdkTurnOptions(),
        persistSession: false,
        systemPrompt: request.systemPrompt,
        settingSources: [],
        tools: [],
        mcpServers: { pi: server },
        env: subscriptionEnvironment(),
        canUseTool: async (toolName, input, permission) => {
          if (toolName !== "mcp__pi__pi_call") {
            return { behavior: "deny", message: `Only the Pi deferred-tool gateway is available, not ${toolName}.` };
          }

          const requestedName = typeof input.name === "string" ? input.name : "";
          const requestedArguments = record(input.arguments);
          if (!availableTools.has(requestedName) || !requestedArguments) {
            deferredError = new Error(`Claude requested an invalid Pi tool call: ${requestedName || "<missing>"}`);
            return { behavior: "deny", message: deferredError.message };
          }

          deferToolCall({
            type: "tool_call",
            id: permission.toolUseID,
            name: requestedName,
            arguments: requestedArguments,
          });
          return { behavior: "deny", message: "Tool execution is deferred to Pi." };
        },
      },
    });

    try {
      for await (const message of sdkQuery) {
        if (deferredError) throw deferredError;
        if (deferredToolCall) {
          yield deferredToolCall;
          return;
        }
        const translated = translateSdkStreamEvent(message);
        if (translated) yield translated;
      }
    } catch (error) {
      if (deferredError) throw deferredError;
      if (deferredToolCall) {
        yield deferredToolCall;
        return;
      }
      throw error;
    }

    if (deferredError) throw deferredError;
    if (deferredToolCall) {
      yield deferredToolCall;
      return;
    }
    yield { type: "done" };
  } catch (error) {
    throw error;
  } finally {
    abortController.abort();
    options?.signal?.removeEventListener("abort", onAbort);
  }
  };
}

export const runClaudeAgentSdk = createClaudeAgentSdkRunner();
