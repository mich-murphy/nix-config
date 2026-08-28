import type { HookCallback, SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import type { Context, Model } from "@earendil-works/pi-ai";

/**
 * Build the minimal Pi context needed by provider adapter tests.
 *
 * @param input - Deliberately partial framework context fixture.
 * @returns The fixture as a Pi provider context.
 */
export function contextFixture(input: unknown): Context {
  // SAFETY: Adapter tests intentionally omit framework-owned timestamps and metadata that the provider never reads. Each test supplies systemPrompt, messages, and tools for the behavior under test.
  return input as Context;
}

/**
 * Build the minimal Claude SDK model needed by provider adapter tests.
 *
 * @param input - Deliberately partial model fixture.
 * @returns The fixture as a Claude SDK model.
 */
export function modelFixture(input: unknown): Model<"claude-sdk"> {
  // SAFETY: Runner tests use only api, provider, id, reasoning, and optional cost fields. Pi normally adds the remaining registry-owned model metadata.
  return input as Model<"claude-sdk">;
}

/**
 * Build an SDK hook input without repeating third-party fixture casts.
 *
 * @param input - Minimal hook event used by the test.
 * @returns The event as the SDK hook input type.
 */
export function hookInputFixture(input: unknown): Parameters<HookCallback>[0] {
  // SAFETY: Tests supply every field read by createPreToolUseHook. Remaining SDK fields are irrelevant to this hook and owned by the third-party runtime.
  return input as Parameters<HookCallback>[0];
}

/**
 * Narrow the SDK query prompt to the streaming-input mode configured by the runner.
 *
 * @param prompt - Prompt captured from SDK query parameters.
 * @returns The streaming SDK user-message input.
 */
export function sdkPromptFixture(prompt: unknown): AsyncIterable<SDKUserMessage> {
  if (typeof prompt !== "object" || prompt === null || !(Symbol.asyncIterator in prompt)) {
    throw new Error("test setup: expected an async SDK prompt");
  }
  // SAFETY: The runner always supplies buildPromptStream(), whose yielded value is SDKUserMessage. The runtime check rejects non-streaming prompt variants.
  return prompt as AsyncIterable<SDKUserMessage>;
}

/**
 * Expose SDK content blocks as records for metadata-only assertions.
 *
 * @param message - SDK user message built by the prompt adapter.
 * @returns Content blocks as unknown-valued records.
 */
export function sdkContentRecords(message: SDKUserMessage | undefined): Array<Record<string, unknown>> {
  if (!message || !Array.isArray(message.message.content)) return [];
  // SAFETY: SDK content is verified as an array. Record values remain unknown, and tests only inspect metadata after narrowing.
  return message.message.content as unknown as Array<Record<string, unknown>>;
}
