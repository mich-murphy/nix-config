import { createSdkMcpServer, query } from "@anthropic-ai/claude-agent-sdk";
import type { Api, Model, SimpleStreamOptions } from "@earendil-works/pi-ai";
import type { AgentRequest, AgentSdkRun, BridgeEvent } from "../bridge";
import { cacheDiagnosticsFromEnvironment, type CacheDiagnosticTracker } from "../cache-diagnostics";
import { createDeferredPiCallTool, createPreToolUseHook, type DeferredCall } from "./deferred-tools";
import { record, resultOutcome, translateSdkStreamEvent, type ResultOutcome } from "./event-translation";
import { buildPromptStream } from "./prompt-stream";
import { subscriptionEnvironment } from "./subscription-environment";

export type RunSdkQuery = (params: Parameters<typeof query>[0]) => AsyncIterable<unknown>;

function effortFor(reasoning: SimpleStreamOptions["reasoning"]): "low" | "medium" | "high" | "xhigh" | "max" | undefined {
  if (!reasoning) return undefined;
  if (reasoning === "minimal") return "low";
  return reasoning;
}

function reasoningOptions(
  model: Model<Api>,
  reasoning: SimpleStreamOptions["reasoning"],
): { effort?: "low" | "medium" | "high" | "xhigh" | "max" } {
  if (!model.reasoning) return {};
  const effort = effortFor(reasoning);
  return effort === undefined ? {} : { effort };
}

export function agentSdkTurnOptions(): { maxTurns?: number } {
  // Pi owns the outer tool loop. A one-turn SDK cap can terminate before the
  // deferred gateway call is handed back to Pi.
  return {};
}

export function createClaudeAgentSdkRunner(
  runSdkQuery: RunSdkQuery = query,
  cacheDiagnostics: CacheDiagnosticTracker | undefined = cacheDiagnosticsFromEnvironment(),
): AgentSdkRun {
  return async function* runClaudeAgentSdk(
    request: AgentRequest,
    model: Model<Api>,
    options?: SimpleStreamOptions,
  ): AsyncGenerator<BridgeEvent> {
    const abortController = new AbortController();
    const onAbort = () => abortController.abort();
    options?.signal?.addEventListener("abort", onAbort, { once: true });

    const diagnosticTurn = cacheDiagnostics?.request(`${model.provider}/${model.id}`, request.promptBlocks);
    let latestUsage: Extract<BridgeEvent, { type: "usage" }> | undefined;
    const availableTools = new Set(request.toolNames);
    const deferredCalls = new Map<string, DeferredCall>();
    let deferredError: Error | undefined;
    const onToolCall = (call: DeferredCall) => {
      if (!deferredCalls.has(call.id)) deferredCalls.set(call.id, call);
    };

    // Denied invalid calls can self-correct inside query(). Cap repeated
    // failures so a model cannot spin forever without producing a valid call.
    const MAX_INVALID_PI_CALLS = 3;
    let invalidCallCount = 0;
    const onInvalidCall = (error: Error) => {
      invalidCallCount += 1;
      if (invalidCallCount <= MAX_INVALID_PI_CALLS) return;
      deferredError ??= error;
    };

    const piCall = createDeferredPiCallTool(request.toolDescription);
    const server = createSdkMcpServer({ name: "pi", version: "0.1.0", tools: [piCall], alwaysLoad: true });

    try {
      const sdkQuery = runSdkQuery({
        prompt: buildPromptStream(request.promptBlocks),
        options: {
          abortController,
          cwd: process.cwd(),
          model: model.id,
          ...reasoningOptions(model, options?.reasoning),
          includePartialMessages: true,
          ...agentSdkTurnOptions(),
          persistSession: false,
          systemPrompt: request.systemPrompt,
          settingSources: [],
          tools: [],
          mcpServers: { pi: server },
          env: subscriptionEnvironment(),
          hooks: { PreToolUse: [{ hooks: [createPreToolUseHook(availableTools, onToolCall, onInvalidCall)] }] },
        },
      });

      let outcome: ResultOutcome | undefined;
      try {
        for await (const message of sdkQuery) {
          // A captured valid call is real progress and takes precedence over
          // invalid attempts that happened in the same turn.
          if (deferredError && deferredCalls.size === 0) throw deferredError;
          const asRecord = record(message);
          if (asRecord?.type === "result") {
            outcome = resultOutcome(asRecord);
            continue;
          }
          const translated = translateSdkStreamEvent(message);
          if (translated?.type === "usage") latestUsage = translated;
          if (translated) yield translated;
        }
      } catch (error) {
        if (deferredError && deferredCalls.size === 0) throw deferredError;
        if (deferredCalls.size === 0) throw error;
      }

      if (diagnosticTurn !== undefined && latestUsage) cacheDiagnostics?.usage(diagnosticTurn, latestUsage);
      if (outcome?.isError) throw new Error(outcome.errorMessage);
      if (deferredError && deferredCalls.size === 0) throw deferredError;
      if (deferredCalls.size > 0) {
        for (const call of deferredCalls.values()) yield { type: "tool_call", ...call };
        return;
      }
      yield { type: "done", reason: outcome?.stopReason ?? "stop" };
    } finally {
      abortController.abort();
      options?.signal?.removeEventListener("abort", onAbort);
    }
  };
}

export const runClaudeAgentSdk = createClaudeAgentSdkRunner();
