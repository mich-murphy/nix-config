import { describe, expect, test } from "bun:test";
import type { Context } from "@earendil-works/pi-ai";
import { buildAgentRequest } from "../bridge";
import { cacheDiagnosticsFromEnvironment, createCacheDiagnosticTracker, type CacheDiagnostic } from "../cache-diagnostics";
import { buildPromptStream } from "../sdk-runner";

async function drain<T>(iterable: AsyncIterable<T>): Promise<T[]> {
  const values: T[] = [];
  for await (const value of iterable) values.push(value);
  return values;
}

describe("cache diagnostics", () => {
  test("reports a byte-identical common prefix for long transcripts containing images without logging content", async () => {
    const longOutput = "stable build output\n".repeat(5_000);
    const base = {
      systemPrompt: "stable system",
      tools: [],
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "inspect screenshot" },
            { type: "image", data: "cGl4ZWxz".repeat(2_000), mimeType: "image/png" },
          ],
        },
        {
          role: "toolResult",
          toolCallId: "call-1",
          toolName: "bash",
          isError: false,
          content: [{ type: "text", text: longOutput }],
        },
      ],
    } as unknown as Context;
    const grown = { ...base, messages: [...base.messages, { role: "user", content: "continue" }] } as unknown as Context;
    const first = buildAgentRequest(base);
    const second = buildAgentRequest(grown);
    const events: CacheDiagnostic[] = [];
    const tracker = createCacheDiagnosticTracker((event) => events.push(event));

    tracker.request("claude-sdk/sonnet", first.promptBlocks);
    tracker.request("claude-sdk/sonnet", second.promptBlocks);

    const secondDiagnostic = events[1];
    expect(secondDiagnostic?.type).toBe("request");
    if (secondDiagnostic?.type !== "request") throw new Error("missing request diagnostic");
    expect(secondDiagnostic.commonPrefixBlocks).toBe(first.promptBlocks.length - 1);
    expect(secondDiagnostic.commonPrefixCharacters).toBeGreaterThan(100_000);
    expect(secondDiagnostic.imageBase64Characters).toBe(16_000);
    expect(JSON.stringify(secondDiagnostic)).not.toContain("stable build output");
    expect(JSON.stringify(secondDiagnostic)).not.toContain("cGl4ZWxz");

    const firstWire = await drain(buildPromptStream(first.promptBlocks));
    const secondWire = await drain(buildPromptStream(second.promptBlocks));
    const firstContent = firstWire[0]?.message.content as unknown as Array<Record<string, unknown>>;
    const secondContent = secondWire[0]?.message.content as unknown as Array<Record<string, unknown>>;
    const withoutCacheMetadata = (block: Record<string, unknown>) => {
      const { cache_control: _cacheControl, ...content } = block;
      return content;
    };
    expect(secondContent.slice(0, firstContent.length - 1).map(withoutCacheMetadata)).toEqual(
      firstContent.slice(0, -1).map(withoutCacheMetadata),
    );
  });

  test("reports wall-clock gap since the previous request so TTL-expiry misses are distinguishable in logs", () => {
    const events: CacheDiagnostic[] = [];
    const tracker = createCacheDiagnosticTracker((event) => events.push(event));

    tracker.request("claude-sdk/sonnet", [{ text: "first", cacheBreakpoint: true }]);
    tracker.request("claude-sdk/sonnet", [{ text: "first" }, { text: "second", cacheBreakpoint: true }]);

    const [first, second] = events;
    if (first?.type !== "request" || second?.type !== "request") throw new Error("missing request diagnostics");
    expect(first.msSincePreviousRequest).toBeUndefined();
    expect(typeof second.msSincePreviousRequest).toBe("number");
    expect(second.msSincePreviousRequest).toBeGreaterThanOrEqual(0);
  });

  test("flags a large low-reuse turn as a possible cache collapse", () => {
    const events: CacheDiagnostic[] = [];
    const tracker = createCacheDiagnosticTracker((event) => events.push(event));
    const turn = tracker.request("claude-sdk/sonnet", [{ text: "x".repeat(100_000), cacheBreakpoint: true }]);
    tracker.usage(turn, { input: 94_337, output: 100, cacheRead: 27_165, cacheWrite: 0 });

    expect(events[1]).toMatchObject({
      type: "usage",
      promptTokens: 121_502,
      cacheReadPercent: 22.36,
      possibleCollapse: true,
    });
  });

  test("is opt-in", () => {
    expect(cacheDiagnosticsFromEnvironment({})).toBeUndefined();
  });
});
