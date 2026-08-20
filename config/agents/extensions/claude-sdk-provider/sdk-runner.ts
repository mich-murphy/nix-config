import { createSdkMcpServer, query, tool } from "@anthropic-ai/claude-agent-sdk";
import type { HookCallback, SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import type { Api, Model, SimpleStreamOptions } from "@earendil-works/pi-ai";
import { z } from "zod";
import type { AgentRequest, AgentSdkRun, BridgeEvent, ImageAttachment, PromptBlock } from "./bridge";
import { cacheDiagnosticsFromEnvironment, type CacheDiagnosticTracker } from "./cache-diagnostics";

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

// The beta header the API requires before honoring ttl: "1h" on a cache_control
// block (see subscriptionEnvironment for why we pin it ourselves).
const EXTENDED_CACHE_TTL_BETA = "extended-cache-ttl-2025-04-11";

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
  // Our cache breakpoint asks for ttl='1h' (see toContentBlocks), but the 1h TTL
  // only takes effect when the request also carries the
  // `extended-cache-ttl-2025-04-11` beta header — and the CLI attaches that header
  // based on its *own* TTL decision (statsig gate + subscription-overage check),
  // not on the ttl we put on the block. When the gate flips off, the API silently
  // degrades the cache to the default 5 minutes: in a real 160-turn session every
  // cache miss had a >5-minute gap from the previous turn and every hit was under
  // ~5 minutes, including misses at 10- and 36-minute gaps that a working 1h TTL
  // would have served. The CLI's decision function checks, in order:
  //   1. FORCE_PROMPT_CACHING_5M — kill switch, forces 5m; must not leak in.
  //   2. ENABLE_PROMPT_CACHING_1H — deterministic opt-in, bypasses the gate.
  //   3. subscription auth + not-on-overage + statsig gate — nondeterministic.
  // Pinning 2 knowingly overrides the CLI's deliberate "no 1h while on overage"
  // cost guard: 1h cache writes bill at 2x base input (vs 1.25x for 5m), but one
  // full-transcript re-write after a 5m expiry costs far more than the 2x premium
  // on each turn's small new suffix, so the pin still wins on cost.
  // The beta header itself is guaranteed via ANTHROPIC_BETAS — the CLI appends
  // that comma-separated list to every request unconditionally — rather than by
  // deleting CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS, which would silently
  // re-enable every OTHER experimental beta the user opted out of, not just
  // extended cache TTL.
  // Escape hatch: PI_CLAUDE_SDK_5M_CACHE=1 skips all of this and restores the
  // CLI's own TTL decision (including Anthropic's FORCE_PROMPT_CACHING_5M kill
  // switch), so a misbehaving 1h-cache beta can be reverted without editing this
  // file.
  if (environment.PI_CLAUDE_SDK_5M_CACHE === "1") return environment;
  delete environment.FORCE_PROMPT_CACHING_5M;
  environment.ENABLE_PROMPT_CACHING_1H = "1";
  const betas = (environment.ANTHROPIC_BETAS ?? "")
    .split(",")
    .map((beta) => beta.trim())
    .filter(Boolean);
  if (!betas.includes(EXTENDED_CACHE_TTL_BETA)) betas.push(EXTENDED_CACHE_TTL_BETA);
  environment.ANTHROPIC_BETAS = betas.join(",");
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
  name: z
    .string()
    .describe('Exact Pi tool name from the available-tools catalog — never "pi_call" itself, which is this gateway\'s own name'),
  arguments: z.record(z.string(), z.unknown()).describe("Arguments matching that Pi tool's input schema"),
};

// A short, concrete sample instead of the whole catalog keeps the deny
// reason readable while still giving the model real names to retry with.
function sampleToolNames(availableTools: ReadonlySet<string>): string {
  const names = [...availableTools].slice(0, 5);
  return names.length > 0 ? names.join(", ") : "(no Pi tools are available this turn)";
}

// "pi_call" is the gateway's own top-level tool name; a model that passes it
// back as the *inner* name is recursing on itself, not naming a real Pi
// tool. That's a recognizable, recurring confusion mode (it killed a real
// production turn — see sdk-runner.ts's PreToolUse hook comment), so it gets
// its own targeted correction instead of a generic "unknown tool" message.
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
// pending tool_use in the turn is resolved. Any other top-level tool name,
// and an invalid pi_call request (unknown inner name, the self-referential
// "pi_call", or missing/malformed arguments), are merely denied with a
// corrective reason: the deny round-trips to the model as a tool result and
// the SDK's own loop keeps running, so one bad request doesn't kill the
// turn and the model can retry within the same query(). onInvalidCall only
// records the failure; it is the caller's job (see
// createClaudeAgentSdkRunner) to decide whether repeated failures should
// eventually become fatal — this hook never ends the turn on its own for an
// invalid inner call.
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
    // An invalid pi_call request is denied, not fatal, on its own (see
    // createPreToolUseHook) — the SDK's own loop keeps running so the model
    // can retry within this same query(). Cap repeated failures anyway: 3
    // invalid attempts is enough for an ordinary self-correction (the model
    // sees the deny reason and fixes its next call) but stops a model stuck
    // resubmitting the same broken request from spinning the turn forever.
    // Only once the cap is exceeded does this become the fatal error thrown
    // below, mirroring the pre-existing deferredError contract for a
    // transport-level failure.
    const MAX_INVALID_PI_CALLS = 3;
    let invalidCallCount = 0;
    const onInvalidCall = (error: Error) => {
      invalidCallCount += 1;
      if (invalidCallCount <= MAX_INVALID_PI_CALLS) return;
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
          hooks: { PreToolUse: [{ hooks: [createPreToolUseHook(availableTools, onToolCall, onInvalidCall)] }] },
        },
      });

      let outcome: ResultOutcome | undefined;
      try {
        for await (const message of sdkQuery) {
          // The cap-exceeded error only becomes fatal if the turn ends with
          // nothing to show for it. A valid pi_call captured alongside (or
          // before) the invalid attempts that tripped the cap is real
          // progress the invalid attempts don't undo — see the post-loop
          // check below for the full precedence this mirrors.
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
        // A hook-deferred call ends the query on its own; only rethrow if
        // nothing was captured (e.g. a transport error, or the cap-exceeded
        // error thrown above when no valid pi_call was ever captured).
        if (deferredError && deferredCalls.size === 0) throw deferredError;
        if (deferredCalls.size === 0) throw error;
      }

      if (diagnosticTurn !== undefined && latestUsage) cacheDiagnostics?.usage(diagnosticTurn, latestUsage);
      // A disagreeing SDK result always wins, checked ahead of the cap.
      if (outcome?.isError) throw new Error(outcome.errorMessage);
      // Cap ranks below both a disagreeing SDK result and any captured
      // call: it only surfaces when the turn produced no valid pi_call at
      // all, i.e. the model spent the whole turn failing to make one.
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
