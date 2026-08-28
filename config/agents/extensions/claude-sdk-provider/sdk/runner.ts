import { createSdkMcpServer, query } from "@anthropic-ai/claude-agent-sdk";
import type { Api, Model, SimpleStreamOptions } from "@earendil-works/pi-ai";
import type { AgentRequest, AgentSdkRun, BridgeEvent } from "../bridge";
import type { CacheDiagnosticTracker } from "../cache-diagnostics";
import { createDeferredPiCallTool, createPreToolUseHook, type DeferredCall } from "./deferred-tools";
import {
  InvalidDeferredCallError,
  InvalidDeferredCallLimitError,
  SdkProtocolError,
  SdkQueryError,
  SdkResultError,
  type SdkRunError,
} from "./errors";
import { record, resultOutcome, translateSdkStreamEvent, type ResultOutcome } from "./event-translation";
import { buildPromptStream } from "./prompt-stream";
import { subscriptionEnvironment } from "./subscription-environment";

/** Injectable Claude Agent SDK query function used by the runner and its tests. */
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

/**
 * Return SDK turn options while leaving Pi in control of the outer tool loop.
 *
 * @returns No `maxTurns` override.
 */
export function agentSdkTurnOptions(): { maxTurns?: number } {
  return {};
}

function isSdkRunError(error: unknown): error is SdkRunError {
  return (
    error instanceof InvalidDeferredCallLimitError ||
    error instanceof SdkProtocolError ||
    error instanceof SdkQueryError ||
    error instanceof SdkResultError
  );
}

/**
 * Create a stateless Claude Agent SDK runner.
 *
 * @param runSdkQuery - SDK query implementation.
 * @param cacheDiagnostics - Optional safe cache diagnostic tracker.
 * @param sdkEnvironment - Sanitized environment captured by the composition root.
 * @returns A runner that emits bridge events and typed failures as values.
 */
export function createClaudeAgentSdkRunner(
  runSdkQuery: RunSdkQuery = query,
  cacheDiagnostics?: CacheDiagnosticTracker,
  sdkEnvironment: Readonly<Record<string, string | undefined>> = subscriptionEnvironment(),
): AgentSdkRun {
  return async function* runClaudeAgentSdk(
    request: AgentRequest,
    model: Model<Api>,
    options?: SimpleStreamOptions,
  ): AsyncGenerator<BridgeEvent> {
    const abortController = new AbortController();
    const onAbort = () => abortController.abort(options?.signal?.reason);
    if (options?.signal?.aborted) abortController.abort(options.signal.reason);
    else options?.signal?.addEventListener("abort", onAbort, { once: true });

    try {
      if (abortController.signal.aborted) {
        yield { type: "failed", error: new SdkQueryError("start", abortController.signal.reason) };
        return;
      }

      const diagnosticTurn = cacheDiagnostics?.request(`${model.provider}/${model.id}`, request.promptBlocks);
      let latestUsage: Extract<BridgeEvent, { type: "usage" }> | undefined;
      const availableTools = new Set(request.toolNames);
      const deferredCalls = new Map<string, DeferredCall>();
      let invalidCallLimitError: InvalidDeferredCallLimitError | undefined;
      const onToolCall = (call: DeferredCall) => {
        if (!deferredCalls.has(call.id)) deferredCalls.set(call.id, call);
      };

      const MAX_INVALID_PI_CALLS = 3;
      let invalidCallCount = 0;
      const onInvalidCall = (error: InvalidDeferredCallError) => {
        invalidCallCount += 1;
        if (invalidCallCount <= MAX_INVALID_PI_CALLS) return;
        invalidCallLimitError ??= new InvalidDeferredCallLimitError(invalidCallCount, error);
        abortController.abort(invalidCallLimitError);
      };

      const piCall = createDeferredPiCallTool(request.toolDescription);
      const server = createSdkMcpServer({ name: "pi", version: "0.1.0", tools: [piCall], alwaysLoad: true });

      let sdkQuery: AsyncIterable<unknown>;
      try {
        sdkQuery = runSdkQuery({
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
            env: { ...sdkEnvironment },
            hooks: { PreToolUse: [{ hooks: [createPreToolUseHook(availableTools, onToolCall, onInvalidCall)] }] },
          },
        });
      } catch (cause) {
        yield { type: "failed", error: new SdkQueryError("start", cause) };
        return;
      }

      let outcome: Extract<ResultOutcome, { _tag: "success" }> | undefined;
      try {
        for await (const message of sdkQuery) {
          if (invalidCallLimitError) {
            yield { type: "failed", error: invalidCallLimitError };
            return;
          }

          const asRecord = record(message);
          if (asRecord?.type === "result") {
            const parsedOutcome = resultOutcome(asRecord);
            if (parsedOutcome._tag === "malformed" || parsedOutcome._tag === "failure") {
              yield { type: "failed", error: parsedOutcome.error };
              return;
            }
            outcome = parsedOutcome;
            continue;
          }

          const translated = translateSdkStreamEvent(message);
          if (translated._tag === "err") {
            yield { type: "failed", error: translated.error };
            return;
          }
          if (translated.value?.type === "usage") latestUsage = translated.value;
          if (translated.value) yield translated.value;
        }
      } catch (cause) {
        const error = invalidCallLimitError ?? (isSdkRunError(cause) ? cause : new SdkQueryError("iterate", cause));
        yield { type: "failed", error };
        return;
      }

      if (invalidCallLimitError) {
        yield { type: "failed", error: invalidCallLimitError };
        return;
      }
      if (!outcome) {
        yield { type: "failed", error: new SdkQueryError("terminal-result", "query ended without a result message") };
        return;
      }
      if (diagnosticTurn !== undefined && latestUsage) cacheDiagnostics?.usage(diagnosticTurn, latestUsage);
      if (deferredCalls.size > 0) {
        if (outcome.terminalReason !== "tool_deferred") {
          yield {
            type: "failed",
            error: new SdkProtocolError(
              "result",
              `captured deferred calls but terminal_reason was ${outcome.terminalReason ?? "missing"}`,
            ),
          };
          return;
        }
        for (const call of deferredCalls.values()) {
          yield { type: "tool_call", id: call.id, name: call.name, arguments: { ...call.arguments } };
        }
        return;
      }
      yield { type: "done", reason: outcome.stopReason };
    } finally {
      abortController.abort();
      options?.signal?.removeEventListener("abort", onAbort);
    }
  };
}

/**
 * Run one turn through a runner composed with the current subscription environment.
 *
 * @param request - Parsed stateless SDK request.
 * @param model - Selected Pi model.
 * @param options - Optional Pi stream settings.
 * @returns Typed bridge events for the turn.
 */
export function runClaudeAgentSdk(
  request: AgentRequest,
  model: Model<Api>,
  options?: SimpleStreamOptions,
): AsyncIterable<BridgeEvent> {
  return createClaudeAgentSdkRunner()(request, model, options);
}
