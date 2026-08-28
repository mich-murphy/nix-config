import { createHash } from "node:crypto";
import type { PromptBlock } from "./bridge";

/** Safe request metadata emitted by cache diagnostics. */
export interface CacheRequestDiagnostic {
  /** Diagnostic record kind. */
  readonly type: "request";
  /** Monotonic request number within the tracker. */
  readonly turn: number;
  /** Provider and model identifier. */
  readonly model: string;
  /** Number of prompt blocks. */
  readonly blocks: number;
  /** Total prompt text characters. */
  readonly textCharacters: number;
  /** Total extracted image base64 characters. */
  readonly imageBase64Characters: number;
  /** Indexes carrying requested cache breakpoints. */
  readonly breakpointBlocks: ReadonlyArray<number>;
  /** Number of byte-identical leading blocks shared with the previous request. */
  readonly commonPrefixBlocks: number;
  /** Character count in the common block prefix. */
  readonly commonPrefixCharacters: number;
  /** Truncated SHA-256 fingerprint of the full content. */
  readonly contentFingerprint: string;
  /** Truncated SHA-256 fingerprint through the final breakpoint. */
  readonly reusablePrefixFingerprint?: string;
  /** Wall-clock gap from the previous request in this tracker. */
  readonly msSincePreviousRequest?: number;
}

/** Safe usage metadata emitted by cache diagnostics. */
export interface CacheUsageDiagnostic {
  /** Diagnostic record kind. */
  readonly type: "usage";
  /** Request number associated with the usage. */
  readonly turn: number;
  /** Uncached input tokens. */
  readonly input: number;
  /** Output tokens. */
  readonly output: number;
  /** Cache-read input tokens. */
  readonly cacheRead: number;
  /** Cache-write input tokens. */
  readonly cacheWrite: number;
  /** Total input-side tokens. */
  readonly promptTokens: number;
  /** Percentage of input-side tokens served from cache. */
  readonly cacheReadPercent: number;
  /** Whether a large prompt had less than 50 percent cache reuse. */
  readonly possibleCollapse: boolean;
}

/** Safe cache diagnostic record. */
export type CacheDiagnostic = CacheRequestDiagnostic | CacheUsageDiagnostic;

/** Destination for safe cache diagnostic records. */
export type CacheDiagnosticSink = (diagnostic: CacheDiagnostic) => void;

function hash(value: string): string {
  return createHash("sha256").update(value).digest("hex").slice(0, 16);
}

function blockPayload(block: PromptBlock): string {
  return JSON.stringify({
    text: block.text,
    images: block.images?.map((image) => ({ mediaType: image.mediaType, data: image.data })) ?? [],
  });
}

function blockCharacters(block: PromptBlock): number {
  return block.text.length + (block.images ?? []).reduce((total, image) => total + image.data.length, 0);
}

/** Stateful cache diagnostic tracker. */
export interface CacheDiagnosticTracker {
  /** Record one outgoing prompt and return its turn number. */
  request(model: string, blocks: ReadonlyArray<PromptBlock>): number;
  /** Record usage for a prior request turn. */
  usage(
    turn: number,
    usage: { readonly input: number; readonly output: number; readonly cacheRead: number; readonly cacheWrite: number },
  ): void;
}

/**
 * Create a tracker that compares consecutive prompt prefixes without logging content.
 *
 * @param sink - Destination for safe diagnostic records.
 * @param now - Injected wall clock, primarily for deterministic tests.
 * @returns Stateful tracker scoped to one provider instance.
 */
export function createCacheDiagnosticTracker(
  sink: CacheDiagnosticSink,
  now: () => number = Date.now,
): CacheDiagnosticTracker {
  let turn = 0;
  let previousPayloads: ReadonlyArray<string> = [];
  let previousRequestAt: number | undefined;
  return {
    request(model, blocks) {
      turn += 1;
      const requestedAt = now();
      const msSincePreviousRequest =
        previousRequestAt === undefined ? undefined : requestedAt - previousRequestAt;
      previousRequestAt = requestedAt;
      const payloads = blocks.map(blockPayload);
      let commonPrefixBlocks = 0;
      while (
        commonPrefixBlocks < previousPayloads.length &&
        commonPrefixBlocks < payloads.length &&
        previousPayloads[commonPrefixBlocks] === payloads[commonPrefixBlocks]
      ) {
        commonPrefixBlocks += 1;
      }
      const commonPrefixCharacters = blocks
        .slice(0, commonPrefixBlocks)
        .reduce((total, block) => total + blockCharacters(block), 0);
      let lastBreakpoint = -1;
      for (let index = blocks.length - 1; index >= 0; index -= 1) {
        if (blocks[index]?.cacheBreakpoint === true) {
          lastBreakpoint = index;
          break;
        }
      }
      const diagnostic: CacheRequestDiagnostic = {
        type: "request",
        turn,
        model,
        blocks: blocks.length,
        textCharacters: blocks.reduce((total, block) => total + block.text.length, 0),
        imageBase64Characters: blocks.reduce(
          (total, block) => total + (block.images ?? []).reduce((sum, image) => sum + image.data.length, 0),
          0,
        ),
        breakpointBlocks: blocks.flatMap((block, index) => (block.cacheBreakpoint ? [index] : [])),
        commonPrefixBlocks,
        commonPrefixCharacters,
        contentFingerprint: hash(payloads.join("\n")),
        ...(lastBreakpoint >= 0
          ? { reusablePrefixFingerprint: hash(payloads.slice(0, lastBreakpoint + 1).join("\n")) }
          : {}),
        ...(msSincePreviousRequest === undefined ? {} : { msSincePreviousRequest }),
      };
      sink(diagnostic);
      previousPayloads = payloads;
      return turn;
    },
    usage(requestTurn, usage) {
      const promptTokens = usage.input + usage.cacheRead + usage.cacheWrite;
      const cacheReadPercent = promptTokens === 0 ? 0 : Math.round((usage.cacheRead / promptTokens) * 10_000) / 100;
      sink({
        type: "usage",
        turn: requestTurn,
        ...usage,
        promptTokens,
        cacheReadPercent,
        possibleCollapse: promptTokens >= 20_000 && usage.cacheRead / promptTokens < 0.5,
      });
    },
  };
}

/**
 * Build an opt-in tracker from parsed environment values.
 *
 * @param environment - Startup environment supplied by the composition root.
 * @returns A stderr tracker when diagnostics are enabled.
 */
export function cacheDiagnosticsFromEnvironment(
  environment: Readonly<Record<string, string | undefined>> = process.env,
): CacheDiagnosticTracker | undefined {
  if (environment.PI_CLAUDE_SDK_CACHE_DIAGNOSTICS !== "1") return undefined;
  return createCacheDiagnosticTracker((diagnostic) => {
    process.stderr.write(`[claude-sdk-cache] ${JSON.stringify(diagnostic)}\n`);
  });
}
