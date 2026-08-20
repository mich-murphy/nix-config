import { createHash } from "node:crypto";
import type { PromptBlock } from "./bridge";

export interface CacheRequestDiagnostic {
  type: "request";
  turn: number;
  model: string;
  blocks: number;
  textCharacters: number;
  imageBase64Characters: number;
  breakpointBlocks: number[];
  commonPrefixBlocks: number;
  commonPrefixCharacters: number;
  contentFingerprint: string;
  reusablePrefixFingerprint?: string;
}

export interface CacheUsageDiagnostic {
  type: "usage";
  turn: number;
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  promptTokens: number;
  cacheReadPercent: number;
  possibleCollapse: boolean;
}

export type CacheDiagnostic = CacheRequestDiagnostic | CacheUsageDiagnostic;
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

export interface CacheDiagnosticTracker {
  request(model: string, blocks: PromptBlock[]): number;
  usage(turn: number, usage: { input: number; output: number; cacheRead: number; cacheWrite: number }): void;
}

export function createCacheDiagnosticTracker(sink: CacheDiagnosticSink): CacheDiagnosticTracker {
  let turn = 0;
  let previousPayloads: string[] = [];
  return {
    request(model, blocks) {
      turn += 1;
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
        reusablePrefixFingerprint:
          lastBreakpoint >= 0 ? hash(payloads.slice(0, lastBreakpoint + 1).join("\n")) : undefined,
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

export function cacheDiagnosticsFromEnvironment(
  environment: Record<string, string | undefined> = process.env,
): CacheDiagnosticTracker | undefined {
  if (environment.PI_CLAUDE_SDK_CACHE_DIAGNOSTICS !== "1") return undefined;
  return createCacheDiagnosticTracker((diagnostic) => {
    process.stderr.write(`[claude-sdk-cache] ${JSON.stringify(diagnostic)}\n`);
  });
}
