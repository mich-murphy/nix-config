import { createSdkMcpServer, query, tool } from "@anthropic-ai/claude-agent-sdk";
import type { HookCallback, SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import type { Api, Model, SimpleStreamOptions } from "@earendil-works/pi-ai";
import { z } from "zod";
import type { AgentRequest, AgentSdkRun, BridgeEvent, ImageAttachment, PromptBlock } from "./bridge";

// query()'s plain-string `prompt` path (see node_modules/@anthropic-ai/claude-agent-sdk/sdk.mjs,
// function IT: `t.write(fe({type:"user",...,message:{role:"user",content:[{type:"text",text:r}]}})...)`)
// always collapses the whole prompt into ONE text content block, with no way to attach
// `cache_control` to part of it. The streaming-input path (`prompt: AsyncIterable<SDKUserMessage>`,
// consumed by Query.streamInput) instead writes whatever MessageParam-shaped content array we hand
// it, so a single yielded message can carry a real Anthropic content-block array with an explicit
// `cache_control` breakpoint. streamInput() closes stdin once the iterable completes, matching the
// existing one-shot-per-Pi-turn subprocess lifecycle.
// Anthropic requires cache_control breakpoint TTLs to be non-increasing through the
// content array. The CLI itself auto-inserts a trailing breakpoint at ttl='1h' on this
// workspace (confirmed live: leaving our own breakpoint's ttl unspecified, or pinned to
// '5m', both got rejected with "a ttl='1h' cache_control block must not come after a
// ttl='5m' cache_control block" once the CLI's own later breakpoint landed at content
// index 22 — one past the block we marked). Matching that ttl keeps the sequence
// non-increasing regardless of what the CLI does elsewhere in the request.
// Anthropic's vision input only accepts these four raster formats. Pi can still hand
// this provider an image outside that set — e.g. pi-coding-agent's
// normalizeToolResultImages keeps a tool's original image block verbatim when its own
// PNG conversion fails — and that image then sits in context.messages permanently.
// Since buildAgentRequest re-serializes and resends the whole transcript on every
// turn, throwing here would turn one such image into a permanent failure for every
// later turn, not just the turn that introduced it. Degrade to a text note instead
// (mirroring the unconditional "omitted from transcript" placeholder this bridge used
// before it forwarded real image bytes) so the turn — and every turn after it —
// proceeds normally.
const SUPPORTED_IMAGE_MEDIA_TYPES = ["image/jpeg", "image/png", "image/gif", "image/webp"] as const;
type AnthropicImageMediaType = (typeof SUPPORTED_IMAGE_MEDIA_TYPES)[number];

function isSupportedImageMediaType(mediaType: string): mediaType is AnthropicImageMediaType {
  return (SUPPORTED_IMAGE_MEDIA_TYPES as readonly string[]).includes(mediaType);
}

// One ImageAttachment becomes one content block: a real Anthropic image block for a
// supported mime type, otherwise a text note. The note text depends only on
// `mediaType`, so it's exactly as deterministic across turns as the real image block
// it replaces — required for the prompt-cache byte-identity invariant on historical
// entries.
function toAnthropicContentBlock(image: ImageAttachment) {
  if (isSupportedImageMediaType(image.mediaType)) {
    return {
      type: "image" as const,
      source: { type: "base64" as const, media_type: image.mediaType, data: image.data },
    };
  }
  return {
    type: "text" as const,
    text: `[Image data omitted from transcript: unsupported mime type "${image.mediaType}"]`,
  };
}

// A PromptBlock expands to its text block plus one content block per attached image,
// in order, so each image's block rides immediately after the JSONL placeholder that
// references it (see ImageAttachment/imageRef in bridge.ts) — whether that block ends
// up a real image or a degraded text note. The cache breakpoint — when set — belongs
// on the *last* of those blocks: it marks "cache everything up to and including
// this", which for an entry with images means the images (or their degraded notes)
// too, not just the entry's text.
function toContentBlocks(block: PromptBlock) {
  const textBlock = { type: "text" as const, text: block.text };
  const imageBlocks = (block.images ?? []).map(toAnthropicContentBlock);
  const blocks = [textBlock, ...imageBlocks];
  if (!block.cacheBreakpoint) return blocks;
  const lastIndex = blocks.length - 1;
  return blocks.map((contentBlock, index) =>
    index === lastIndex
      ? { ...contentBlock, cache_control: { type: "ephemeral" as const, ttl: "1h" as const } }
      : contentBlock,
  );
}

export async function* buildPromptStream(promptBlocks: PromptBlock[]): AsyncGenerator<SDKUserMessage> {
  yield {
    type: "user",
    session_id: "",
    parent_tool_use_id: null,
    message: { role: "user", content: promptBlocks.flatMap(toContentBlocks) },
  };
}

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
    if (delta?.type === "thinking_delta" && typeof delta.thinking === "string") {
      return { type: "thinking_delta", text: delta.thinking };
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

interface ResultOutcome {
  isError: boolean;
  stopReason: "stop" | "length";
  errorMessage?: string;
}

// terminal_reason: "tool_deferred_unavailable" means the hook asked to defer
// but the SDK couldn't honor it, so it's an error even when is_error is false —
// the caller must not hand Pi the hook's earlier capture as a clean defer.
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

export interface DeferredCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

interface DeferredPiCallInput {
  name: string;
  arguments: Record<string, unknown>;
}

// Pure schema, no per-turn state: build the zod shape once and reuse it for
// every turn instead of re-deriving it on every call to
// createClaudeAgentSdkRunner()'s generator.
const PI_CALL_INPUT_SCHEMA = {
  name: z.string().describe("Exact Pi tool name from the available-tools catalog"),
  arguments: z.record(z.string(), z.unknown()).describe("Arguments matching that Pi tool's input schema"),
};

function validateDeferredCall(
  availableTools: ReadonlySet<string>,
  input: unknown,
): { ok: true; call: Omit<DeferredCall, "id"> } | { ok: false; error: Error } {
  const fields = record(input);
  const requestedName = typeof fields?.name === "string" ? fields.name : "";
  const requestedArguments = record(fields?.arguments);
  if (!availableTools.has(requestedName) || !requestedArguments) {
    return {
      ok: false,
      error: new Error(`Claude requested an invalid Pi tool call: ${requestedName || "<missing>"}`),
    };
  }
  return { ok: true, call: { name: requestedName, arguments: requestedArguments } };
}

// tool() requires a handler, but PreToolUse defer normally resolves the call
// before this can run. If the SDK invokes it anyway, fail loudly instead of
// faking success, and don't forward to Pi — this handler has no typed
// tool_use_id, so forwarding risks double-executing a call the hook already
// handled under a different id.
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

// The handler has no per-turn captured state, so one instance can serve every
// turn instead of allocating a fresh closure per call to
// createClaudeAgentSdkRunner()'s generator.
const DEFERRED_PI_CALL_HANDLER = createDeferredPiCallHandler();

// permissionDecision: "defer" ends query() cleanly (terminal_reason:
// "tool_deferred") with no abort needed — the query finishes once every
// pending tool_use in the turn is resolved. Any other tool name is denied.
export function createPreToolUseHook(
  availableTools: ReadonlySet<string>,
  onToolCall: (toolCall: DeferredCall) => void,
  onError: (error: Error) => void,
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
      onError(validated.error);
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

export function createClaudeAgentSdkRunner(runSdkQuery: RunSdkQuery = query): AgentSdkRun {
  return async function* runClaudeAgentSdk(
    request: AgentRequest,
    model: Model<Api>,
    options?: SimpleStreamOptions,
  ): AsyncGenerator<BridgeEvent> {
    const abortController = new AbortController();
    const onAbort = () => abortController.abort();
    options?.signal?.addEventListener("abort", onAbort, { once: true });

    const availableTools = new Set(request.toolNames);
    const deferredCalls = new Map<string, DeferredCall>();
    let deferredError: Error | undefined;
    const onToolCall = (call: DeferredCall) => {
      if (!deferredCalls.has(call.id)) deferredCalls.set(call.id, call);
    };
    const onError = (error: Error) => {
      deferredError ??= error;
    };

    const piCall = tool("pi_call", request.toolDescription, PI_CALL_INPUT_SCHEMA, DEFERRED_PI_CALL_HANDLER);
    const server = createSdkMcpServer({ name: "pi", version: "0.1.0", tools: [piCall], alwaysLoad: true });

    try {
      const sdkQuery = runSdkQuery({
        prompt: buildPromptStream(request.promptBlocks),
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
          hooks: { PreToolUse: [{ hooks: [createPreToolUseHook(availableTools, onToolCall, onError)] }] },
        },
      });

      let outcome: ResultOutcome | undefined;
      try {
        for await (const message of sdkQuery) {
          if (deferredError) throw deferredError;
          const asRecord = record(message);
          if (asRecord?.type === "result") {
            outcome = resultOutcome(asRecord);
            continue;
          }
          const translated = translateSdkStreamEvent(message);
          if (translated) yield translated;
        }
      } catch (error) {
        // A hook-deferred call ends the query on its own; only rethrow if
        // nothing was captured (e.g. a transport error).
        if (deferredError) throw deferredError;
        if (deferredCalls.size === 0) throw error;
      }

      if (deferredError) throw deferredError;
      if (outcome?.isError) throw new Error(outcome.errorMessage);
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
