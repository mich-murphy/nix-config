import { describe, expect, test } from "bun:test";
import type { SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import type { Context, Model } from "@earendil-works/pi-ai";
import { buildAgentRequest, createAgentSdkStream, serializeConversation, type AgentRequest, type BridgeEvent } from "../bridge";
import { models } from "../index";
import {
  agentSdkTurnOptions,
  buildPromptStream,
  createClaudeAgentSdkRunner,
  createDeferredPiCallHandler,
  createPreToolUseHook,
  resultOutcome,
  subscriptionEnvironment,
  translateSdkStreamEvent,
  type DeferredCall,
  type RunSdkQuery,
} from "../sdk-runner";

async function drain<T>(iterable: AsyncIterable<T>): Promise<T[]> {
  const items: T[] = [];
  for await (const item of iterable) items.push(item);
  return items;
}

describe("serializeConversation", () => {
  test("starts each Pi turn in a stateless SDK session with the complete transcript", async () => {
    const sdkRequests: Array<Parameters<RunSdkQuery>[0]> = [];
    const runSdkQuery = (params: (typeof sdkRequests)[number]) => {
      sdkRequests.push(params);
      return (async function* () {})();
    };
    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;
    const firstRequest: AgentRequest = {
      systemPrompt: "stable system prompt",
      promptBlocks: [{ text: "complete transcript: first turn" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: ['{"role":"user","content":"Read package.json"}'],
    };
    const secondRequest: AgentRequest = {
      ...firstRequest,
      promptBlocks: [
        { text: "complete transcript: first turn" },
        { text: "complete transcript: second turn" },
      ],
      conversationEntries: [
        ...firstRequest.conversationEntries,
        '{"role":"assistant","content":[{"type":"toolCall","id":"call-1","name":"read","arguments":{"path":"package.json"}}]}',
        '{"role":"toolResult","toolCallId":"call-1","toolName":"read","isError":false,"content":[{"type":"text","text":"{}"}]}',
      ],
    };

    for await (const _event of runner(firstRequest, model)) {}
    for await (const _event of runner(secondRequest, model)) {}

    const drainedPrompts = await Promise.all(
      sdkRequests.map(({ prompt }) => drain(prompt as AsyncIterable<SDKUserMessage>)),
    );
    expect(drainedPrompts.map((messages) => messages.length)).toEqual([1, 1]);
    const textsOf = (message: SDKUserMessage) =>
      (message.message.content as Array<{ text: string }>).map((block) => block.text);
    expect(drainedPrompts.map(([message]) => textsOf(message!))).toEqual([
      firstRequest.promptBlocks.map((block) => block.text),
      secondRequest.promptBlocks.map((block) => block.text),
    ]);
    expect(sdkRequests.map(({ options }) => options?.persistSession)).toEqual([false, false]);
    expect(sdkRequests.every(({ options }) => options?.resume === undefined)).toBe(true);
    expect(sdkRequests.every(({ options }) => options?.sessionId === undefined)).toBe(true);
  });

  test("preserves text, tool calls, and tool results as a JSONL transcript", () => {
    const context = {
      systemPrompt: "Use Pi's tools.",
      messages: [
        { role: "user", content: "Read the package file" },
        {
          role: "assistant",
          content: [
            { type: "thinking", thinking: "private reasoning", thinkingSignature: "signature" },
            { type: "toolCall", id: "call-1", name: "read", arguments: { path: "package.json" } },
          ],
        },
        {
          role: "toolResult",
          toolCallId: "call-1",
          toolName: "read",
          isError: false,
          content: [{ type: "text", text: '{"name":"demo"}' }],
        },
      ],
      tools: [],
    } as unknown as Context;

    expect(serializeConversation(context)).toBe(
      [
        '{"role":"user","content":[{"type":"text","text":"Read the package file"}]}',
        '{"role":"assistant","content":[{"type":"toolCall","id":"call-1","name":"read","arguments":{"path":"package.json"}}]}',
        '{"role":"toolResult","toolCallId":"call-1","toolName":"read","isError":false,"content":[{"type":"text","text":"{\\"name\\":\\"demo\\"}"}]}',
      ].join("\n"),
    );
  });

  test("streams SDK text and usage through Pi's provider event contract", async () => {
    const context = {
      systemPrompt: "Be concise.",
      messages: [{ role: "user", content: "Hello" }],
      tools: [],
    } as unknown as Context;
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "text_delta", text: "Hello" };
      yield { type: "text_delta", text: " from Claude" };
      yield { type: "usage", input: 12, output: 3, cacheRead: 4, cacheWrite: 0 };
      yield { type: "done", reason: "stop" };
    };

    const events = [];
    for await (const event of createAgentSdkStream(model, context, {}, run)) events.push(event);

    expect(events.map((event) => event.type)).toEqual([
      "start",
      "text_start",
      "text_delta",
      "text_delta",
      "text_end",
      "done",
    ]);
    const done = events.at(-1);
    expect(done?.type).toBe("done");
    if (done?.type === "done") {
      expect(done.reason).toBe("stop");
      expect(done.message.content).toEqual([{ type: "text", text: "Hello from Claude" }]);
      expect(done.message.usage).toMatchObject({ input: 12, output: 3, cacheRead: 4, totalTokens: 19 });
    }
  });

  test("streams thinking deltas as a distinct content block", async () => {
    const context = {
      systemPrompt: "Be concise.",
      messages: [{ role: "user", content: "Hello" }],
      tools: [],
    } as unknown as Context;
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "thinking_delta", text: "Let me " };
      yield { type: "thinking_delta", text: "think." };
      yield { type: "text_delta", text: "Answer." };
      yield { type: "done", reason: "stop" };
    };

    const events = [];
    for await (const event of createAgentSdkStream(model, context, {}, run)) events.push(event);

    expect(events.map((event) => event.type)).toEqual([
      "start",
      "thinking_start",
      "thinking_delta",
      "thinking_delta",
      "thinking_end",
      "text_start",
      "text_delta",
      "text_end",
      "done",
    ]);
    const done = events.at(-1);
    if (done?.type === "done") {
      expect(done.message.content).toEqual([
        { type: "thinking", thinking: "Let me think." },
        { type: "text", text: "Answer." },
      ]);
    }
  });

  test("ends the Pi turn with a deferred tool call from the SDK gateway", async () => {
    const context = {
      systemPrompt: "Use tools when needed.",
      messages: [{ role: "user", content: "Read package.json" }],
      tools: [],
    } as unknown as Context;
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "tool_call", id: "tool-1", name: "read", arguments: { path: "package.json" } };
    };

    const events = [];
    for await (const event of createAgentSdkStream(model, context, {}, run)) events.push(event);

    expect(events.map((event) => event.type)).toEqual(["start", "toolcall_start", "toolcall_end", "done"]);
    const done = events.at(-1);
    expect(done?.type).toBe("done");
    if (done?.type === "done") {
      expect(done.reason).toBe("toolUse");
      expect(done.message.content).toEqual([
        { type: "toolCall", id: "tool-1", name: "read", arguments: { path: "package.json" } },
      ]);
    }
  });

  test("keeps every tool call the model batches into one turn, not just the first", async () => {
    const context = {
      systemPrompt: "Use tools when needed.",
      messages: [{ role: "user", content: "Read both files" }],
      tools: [],
    } as unknown as Context;
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "tool_call", id: "tool-1", name: "read", arguments: { path: "package.json" } };
      yield { type: "tool_call", id: "tool-2", name: "read", arguments: { path: "README.md" } };
    };

    const events = [];
    for await (const event of createAgentSdkStream(model, context, {}, run)) events.push(event);

    const done = events.at(-1);
    expect(done?.type).toBe("done");
    if (done?.type === "done") {
      expect(done.reason).toBe("toolUse");
      expect(done.message.content).toEqual([
        { type: "toolCall", id: "tool-1", name: "read", arguments: { path: "package.json" } },
        { type: "toolCall", id: "tool-2", name: "read", arguments: { path: "README.md" } },
      ]);
    }
  });

  test("reports a length stop reason when the SDK ends the turn at max_tokens", async () => {
    const context = {
      systemPrompt: "Be concise.",
      messages: [{ role: "user", content: "Hello" }],
      tools: [],
    } as unknown as Context;
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "text_delta", text: "Truncated" };
      yield { type: "done", reason: "length" };
    };

    const events = [];
    for await (const event of createAgentSdkStream(model, context, {}, run)) events.push(event);

    const done = events.at(-1);
    expect(done?.type).toBe("done");
    if (done?.type === "done") expect(done.reason).toBe("length");
  });

  test("removes API and cloud-provider credentials from the subscription subprocess", () => {
    const environment = subscriptionEnvironment({
      PATH: "/bin",
      ANTHROPIC_API_KEY: "api-key",
      ANTHROPIC_AUTH_TOKEN: "auth-token",
      CLAUDE_CODE_USE_BEDROCK: "1",
      CLAUDE_CODE_USE_VERTEX: "1",
      CLAUDE_CODE_USE_FOUNDRY: "1",
    });

    expect(environment.PATH).toBe("/bin");
    expect(environment.CLAUDE_AGENT_SDK_CLIENT_APP).toBe("pi-coding-agent-provider/0.1.0");
    expect(environment.ANTHROPIC_API_KEY).toBeUndefined();
    expect(environment.ANTHROPIC_AUTH_TOKEN).toBeUndefined();
    expect(environment.CLAUDE_CODE_USE_BEDROCK).toBeUndefined();
    expect(environment.CLAUDE_CODE_USE_VERTEX).toBeUndefined();
    expect(environment.CLAUDE_CODE_USE_FOUNDRY).toBeUndefined();
  });

  test("pins the 1h prompt-cache TTL opt-in and beta header so the extended cache TTL is deterministic", () => {
    const environment = subscriptionEnvironment({
      FORCE_PROMPT_CACHING_5M: "1",
      ENABLE_PROMPT_CACHING_1H: undefined,
    });

    expect(environment.ENABLE_PROMPT_CACHING_1H).toBe("1");
    expect(environment.FORCE_PROMPT_CACHING_5M).toBeUndefined();
    expect(environment.ANTHROPIC_BETAS).toBe("extended-cache-ttl-2025-04-11");
  });

  test("appends the extended-cache beta without clobbering existing betas or the experimental-betas opt-out", () => {
    const environment = subscriptionEnvironment({
      ANTHROPIC_BETAS: "context-1m-2025-08-07, extended-cache-ttl-2025-04-11",
      CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS: "1",
    });

    expect(environment.ANTHROPIC_BETAS).toBe("context-1m-2025-08-07,extended-cache-ttl-2025-04-11");
    expect(environment.CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS).toBe("1");
  });

  test("PI_CLAUDE_SDK_5M_CACHE=1 restores the CLI's own cache-TTL decision as an escape hatch", () => {
    const environment = subscriptionEnvironment({
      PI_CLAUDE_SDK_5M_CACHE: "1",
      FORCE_PROMPT_CACHING_5M: "1",
    });

    expect(environment.ENABLE_PROMPT_CACHING_1H).toBeUndefined();
    expect(environment.FORCE_PROMPT_CACHING_5M).toBe("1");
    expect(environment.ANTHROPIC_BETAS).toBeUndefined();
  });

  test("lets Pi own the multi-turn tool loop instead of capping each SDK query at one model turn", () => {
    expect(agentSdkTurnOptions()).toEqual({});
  });

  test("throws on an SDK result error even after the PreToolUse hook already captured a deferred call", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      await hook(
        {
          hook_event_name: "PreToolUse",
          tool_name: "mcp__pi__pi_call",
          tool_use_id: "toolu_err",
          tool_input: { name: "read", arguments: { path: "package.json" } },
        } as Parameters<typeof hook>[0],
        "toolu_err",
        { signal: new AbortController().signal },
      );
      yield {
        type: "result",
        is_error: true,
        stop_reason: null,
        terminal_reason: "tool_deferred_unavailable",
        errors: ["the SDK could not honor the deferred tool call"],
      };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    await expect(
      (async () => {
        for await (const event of runner(request, model)) events.push(event);
      })(),
    ).rejects.toThrow("the SDK could not honor the deferred tool call");
    expect(events).toEqual([]);
  });

  test("yields the hook-captured tool call once the SDK result confirms a clean defer", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      await hook(
        {
          hook_event_name: "PreToolUse",
          tool_name: "mcp__pi__pi_call",
          tool_use_id: "toolu_ok",
          tool_input: { name: "read", arguments: { path: "package.json" } },
        } as Parameters<typeof hook>[0],
        "toolu_ok",
        { signal: new AbortController().signal },
      );
      yield { type: "result", is_error: false, stop_reason: null, terminal_reason: "tool_deferred" };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    for await (const event of runner(request, model)) events.push(event);

    expect(events).toEqual([{ type: "tool_call", id: "toolu_ok", name: "read", arguments: { path: "package.json" } }]);
  });

  test("keeps each turn's allowed-tool set independent even though the MCP schema and handler are shared module singletons", async () => {
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const makeRun =
      (toolUseId: string, name: string): RunSdkQuery =>
      async function* (params) {
        const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
        if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
        await hook(
          {
            hook_event_name: "PreToolUse",
            tool_name: "mcp__pi__pi_call",
            tool_use_id: toolUseId,
            tool_input: { name, arguments: {} },
          } as Parameters<typeof hook>[0],
          toolUseId,
          { signal: new AbortController().signal },
        );
        yield { type: "result", is_error: false, stop_reason: null, terminal_reason: "tool_deferred" };
      };

    const runner = createClaudeAgentSdkRunner(makeRun("toolu_a", "read"));
    const firstTurnEvents: BridgeEvent[] = [];
    for await (const event of runner(
      {
        systemPrompt: "s",
        promptBlocks: [{ text: "p" }],
        toolDescription: "turn one tools",
        toolNames: ["read"],
        conversationEntries: [],
      },
      model,
    )) {
      firstTurnEvents.push(event);
    }
    expect(firstTurnEvents).toEqual([{ type: "tool_call", id: "toolu_a", name: "read", arguments: {} }]);

    // Second, independent turn requesting a tool that only this turn's catalog allows.
    // If the hoisted schema/handler singletons leaked state from the first turn, this
    // would incorrectly deny "write" or incorrectly allow "read".
    let capturedError: Error | undefined;
    const secondRunner = createClaudeAgentSdkRunner(makeRun("toolu_b", "write"));
    const secondRequest: AgentRequest = {
      systemPrompt: "s",
      promptBlocks: [{ text: "p" }],
      toolDescription: "turn two tools",
      toolNames: ["write"],
      conversationEntries: [],
    };
    const secondTurnEvents: BridgeEvent[] = [];
    try {
      for await (const event of secondRunner(secondRequest, model)) secondTurnEvents.push(event);
    } catch (error) {
      capturedError = error as Error;
    }
    expect(capturedError).toBeUndefined();
    expect(secondTurnEvents).toEqual([{ type: "tool_call", id: "toolu_b", name: "write", arguments: {} }]);

    // And a tool name valid in turn one but not turn two must still be denied in turn
    // two — but denial alone is non-fatal (see the invalid-pi_call contract below), so
    // this ends the turn cleanly with "done" rather than throwing.
    const thirdRunner = createClaudeAgentSdkRunner(makeRun("toolu_c", "read"));
    const thirdRequest: AgentRequest = {
      systemPrompt: "s",
      promptBlocks: [{ text: "p" }],
      toolDescription: "turn three tools",
      toolNames: ["write"],
      conversationEntries: [],
    };
    const thirdEvents: BridgeEvent[] = [];
    for await (const event of thirdRunner(thirdRequest, model)) thirdEvents.push(event);
    expect(thirdEvents).toEqual([{ type: "done", reason: "stop" }]);
  });

  test("lets the model retry after an invalid pi_call within the same query instead of ending the turn", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      // First attempt: the recognizable "pi_call as its own inner name" mistake.
      await hook(
        {
          hook_event_name: "PreToolUse",
          tool_name: "mcp__pi__pi_call",
          tool_use_id: "toolu_bad",
          tool_input: { name: "pi_call", arguments: {} },
        } as Parameters<typeof hook>[0],
        "toolu_bad",
        { signal: new AbortController().signal },
      );
      // The model corrects itself later in the same query() call.
      await hook(
        {
          hook_event_name: "PreToolUse",
          tool_name: "mcp__pi__pi_call",
          tool_use_id: "toolu_good",
          tool_input: { name: "read", arguments: { path: "package.json" } },
        } as Parameters<typeof hook>[0],
        "toolu_good",
        { signal: new AbortController().signal },
      );
      yield { type: "result", is_error: false, stop_reason: null, terminal_reason: "tool_deferred" };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    for await (const event of runner(request, model)) events.push(event);

    expect(events).toEqual([{ type: "tool_call", id: "toolu_good", name: "read", arguments: { path: "package.json" } }]);
  });

  test("caps repeated invalid pi_call attempts instead of letting the model loop forever, then surfaces a real error", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      // One more than MAX_INVALID_PI_CALLS (3) in createClaudeAgentSdkRunner — the
      // model never self-corrects, so this must eventually become a real error
      // instead of denying forever.
      for (let attempt = 0; attempt < 4; attempt += 1) {
        await hook(
          {
            hook_event_name: "PreToolUse",
            tool_name: "mcp__pi__pi_call",
            tool_use_id: `toolu_bad_${attempt}`,
            tool_input: { name: "pi_call", arguments: {} },
          } as Parameters<typeof hook>[0],
          `toolu_bad_${attempt}`,
          { signal: new AbortController().signal },
        );
      }
      yield { type: "result", is_error: false, stop_reason: null, terminal_reason: "tool_deferred" };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    await expect(
      (async () => {
        for await (const event of runner(request, model)) events.push(event);
      })(),
    ).rejects.toThrow(/pi_call.*is this gateway's own name/);
    expect(events).toEqual([]);
  });

  test("tolerates exactly MAX_INVALID_PI_CALLS invalid attempts before a valid one — the cap is inclusive, not exclusive", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      // Exactly MAX_INVALID_PI_CALLS (3) invalid attempts — one short of the
      // cap trip point — followed by a valid request in the same query().
      for (let attempt = 0; attempt < 3; attempt += 1) {
        await hook(
          {
            hook_event_name: "PreToolUse",
            tool_name: "mcp__pi__pi_call",
            tool_use_id: `toolu_bad_${attempt}`,
            tool_input: { name: "pi_call", arguments: {} },
          } as Parameters<typeof hook>[0],
          `toolu_bad_${attempt}`,
          { signal: new AbortController().signal },
        );
      }
      await hook(
        {
          hook_event_name: "PreToolUse",
          tool_name: "mcp__pi__pi_call",
          tool_use_id: "toolu_good",
          tool_input: { name: "read", arguments: { path: "package.json" } },
        } as Parameters<typeof hook>[0],
        "toolu_good",
        { signal: new AbortController().signal },
      );
      yield { type: "result", is_error: false, stop_reason: null, terminal_reason: "tool_deferred" };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    for await (const event of runner(request, model)) events.push(event);

    expect(events).toEqual([{ type: "tool_call", id: "toolu_good", name: "read", arguments: { path: "package.json" } }]);
  });

  test("hands a captured valid pi_call to Pi even when other invalid attempts in the same turn exceed the cap", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      // 4 invalid attempts — one past MAX_INVALID_PI_CALLS (3) — plus a
      // valid request batched into the same turn (e.g. parallel tool use).
      // The cap trips, but the valid call was already captured, so it must
      // still reach Pi instead of the turn hard-erroring.
      for (let attempt = 0; attempt < 4; attempt += 1) {
        await hook(
          {
            hook_event_name: "PreToolUse",
            tool_name: "mcp__pi__pi_call",
            tool_use_id: `toolu_bad_${attempt}`,
            tool_input: { name: "pi_call", arguments: {} },
          } as Parameters<typeof hook>[0],
          `toolu_bad_${attempt}`,
          { signal: new AbortController().signal },
        );
      }
      await hook(
        {
          hook_event_name: "PreToolUse",
          tool_name: "mcp__pi__pi_call",
          tool_use_id: "toolu_good",
          tool_input: { name: "read", arguments: { path: "package.json" } },
        } as Parameters<typeof hook>[0],
        "toolu_good",
        { signal: new AbortController().signal },
      );
      yield { type: "result", is_error: false, stop_reason: null, terminal_reason: "tool_deferred" };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    for await (const event of runner(request, model)) events.push(event);

    expect(events).toEqual([{ type: "tool_call", id: "toolu_good", name: "read", arguments: { path: "package.json" } }]);
  });

  // The "tolerates exactly MAX_INVALID_PI_CALLS..." test above (3 invalid +
  // 1 valid) cannot pin the cap value 3 on its own: once a valid call is
  // captured, "captured calls win over the cap" (the test right above this
  // one) makes that scenario succeed under ANY cap value, not just 3. This
  // test isolates the fencepost by never capturing a valid call at all, so
  // the cap's own pass/fail boundary is the only thing that can end the
  // turn cleanly. Paired with the existing "caps repeated invalid pi_call
  // attempts..." test above (4 invalid, 0 valid, expects a throw), the two
  // together pin the cap at exactly 3: this test fails if the cap drops to
  // 2 (3 invalid attempts would then exceed it and throw instead of ending
  // cleanly), and that test fails if the cap rises to 4 (4 invalid attempts
  // would then no longer exceed it, so it would end cleanly instead of
  // throwing). Verified both directions by hand — see the round-3 fix
  // report for the constant-swap evidence.
  test("ends a turn cleanly at exactly MAX_INVALID_PI_CALLS invalid attempts with no valid pi_call ever captured", async () => {
    const request: AgentRequest = {
      systemPrompt: "Use tools when needed.",
      promptBlocks: [{ text: "Read package.json" }],
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: [],
    };
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;

    const runSdkQuery: RunSdkQuery = async function* (params) {
      const hook = params.options?.hooks?.PreToolUse?.[0]?.hooks?.[0];
      if (!hook) throw new Error("test setup: PreToolUse hook missing from SDK query options");
      // Exactly MAX_INVALID_PI_CALLS (3) invalid attempts, at the boundary,
      // not past it — and no valid pi_call at all this turn. The query
      // then ends the ordinary way (a normal, non-error, non-defer
      // result), never signaling a cap trip on its own.
      for (let attempt = 0; attempt < 3; attempt += 1) {
        await hook(
          {
            hook_event_name: "PreToolUse",
            tool_name: "mcp__pi__pi_call",
            tool_use_id: `toolu_bad_${attempt}`,
            tool_input: { name: "pi_call", arguments: {} },
          } as Parameters<typeof hook>[0],
          `toolu_bad_${attempt}`,
          { signal: new AbortController().signal },
        );
      }
      yield { type: "result", is_error: false, stop_reason: "end_turn", terminal_reason: "completed" };
    };

    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const events: BridgeEvent[] = [];
    for await (const event of runner(request, model)) events.push(event);

    expect(events).toEqual([{ type: "done", reason: "stop" }]);
  });

  test("fails loudly instead of faking a successful defer when the SDK invokes the MCP gateway directly", async () => {
    const handler = createDeferredPiCallHandler();

    const result = await handler({ name: "read", arguments: { path: "package.json" } });

    expect(result.isError).toBe(true);
    expect(result.content).toHaveLength(1);
    expect(result.content[0]?.text).not.toContain("Tool execution is deferred to Pi.");
    expect(result.content[0]?.text).toContain("not honored");
  });

  test("defers the pi_call gateway tool via a PreToolUse hook instead of denying and aborting", async () => {
    let deferred: DeferredCall | undefined;
    const hook = createPreToolUseHook(
      new Set(["read"]),
      (toolCall) => {
        deferred = toolCall;
      },
      () => {},
    );

    const output = await hook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "mcp__pi__pi_call",
        tool_use_id: "toolu_1",
        tool_input: { name: "read", arguments: { path: "package.json" } },
      } as Parameters<typeof hook>[0],
      "toolu_1",
      { signal: new AbortController().signal },
    );

    expect(deferred).toEqual({ id: "toolu_1", name: "read", arguments: { path: "package.json" } });
    expect(output).toEqual({
      hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "defer" },
    });
  });

  test("denies any tool other than the pi_call gateway from the PreToolUse hook", async () => {
    const hook = createPreToolUseHook(new Set(["read"]), () => {}, () => {});

    const output = await hook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "Bash",
        tool_use_id: "toolu_2",
        tool_input: { command: "ls" },
      } as Parameters<typeof hook>[0],
      "toolu_2",
      { signal: new AbortController().signal },
    );

    expect(output).toMatchObject({
      hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "deny" },
    });
  });

  test("denies (not fatally) an unknown inner tool name from the PreToolUse hook, naming real tools to retry with", async () => {
    let capturedError: Error | undefined;
    const hook = createPreToolUseHook(new Set(["read"]), () => {}, (error) => {
      capturedError = error;
    });

    const output = await hook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "mcp__pi__pi_call",
        tool_use_id: "toolu_3",
        tool_input: { name: "missing_tool", arguments: {} },
      } as Parameters<typeof hook>[0],
      "toolu_3",
      { signal: new AbortController().signal },
    );

    expect(capturedError?.message).toBe(
      'Invalid Pi tool call: "missing_tool" is not a recognized Pi tool. Available tools: read.',
    );
    expect(output).toMatchObject({
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        permissionDecision: "deny",
        permissionDecisionReason: capturedError?.message,
      },
    });
  });

  test("gives a targeted correction when the model passes pi_call as its own inner name", async () => {
    let capturedError: Error | undefined;
    const hook = createPreToolUseHook(new Set(["read", "write"]), () => {}, (error) => {
      capturedError = error;
    });

    const output = await hook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "mcp__pi__pi_call",
        tool_use_id: "toolu_4",
        tool_input: { name: "pi_call", arguments: {} },
      } as Parameters<typeof hook>[0],
      "toolu_4",
      { signal: new AbortController().signal },
    );

    expect(capturedError?.message).toBe(
      'Invalid Pi tool call: "pi_call" is this gateway\'s own name, not a Pi tool — do not pass it as the "name" field. ' +
        "Pass the target Pi tool's name instead, e.g. read, write.",
    );
    expect(output).toMatchObject({
      hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "deny" },
    });
  });

  test("reports missing/malformed arguments distinctly from an unknown tool name", async () => {
    let capturedError: Error | undefined;
    const hook = createPreToolUseHook(new Set(["read"]), () => {}, (error) => {
      capturedError = error;
    });

    const output = await hook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "mcp__pi__pi_call",
        tool_use_id: "toolu_5",
        tool_input: { name: "read" },
      } as Parameters<typeof hook>[0],
      "toolu_5",
      { signal: new AbortController().signal },
    );

    expect(capturedError?.message).toBe(
      'Invalid Pi tool call: "arguments" must be an object matching "read"\'s input schema.',
    );
    expect(output).toMatchObject({
      hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "deny" },
    });
  });

  test("translates official Agent SDK stream events without depending on private endpoints", () => {
    expect(
      translateSdkStreamEvent({
        type: "stream_event",
        event: { type: "content_block_delta", delta: { type: "text_delta", text: "Hi" } },
      }),
    ).toEqual({ type: "text_delta", text: "Hi" });

    expect(
      translateSdkStreamEvent({
        type: "stream_event",
        event: { type: "content_block_delta", delta: { type: "thinking_delta", thinking: "Hmm" } },
      }),
    ).toEqual({ type: "thinking_delta", text: "Hmm" });

    expect(
      translateSdkStreamEvent({
        type: "stream_event",
        event: {
          type: "message_delta",
          usage: {
            input_tokens: 10,
            output_tokens: 2,
            cache_read_input_tokens: 3,
            cache_creation_input_tokens: 1,
          },
        },
      }),
    ).toEqual({ type: "usage", input: 10, output: 2, cacheRead: 3, cacheWrite: 1 });
  });

  test("maps a clean SDK result to a stop or length reason", () => {
    expect(resultOutcome({ is_error: false, stop_reason: "end_turn" })).toEqual({
      isError: false,
      stopReason: "stop",
    });
    expect(resultOutcome({ is_error: false, stop_reason: "max_tokens" })).toEqual({
      isError: false,
      stopReason: "length",
    });
  });

  test("surfaces the SDK's own error result instead of silently reporting an empty stop", () => {
    expect(
      resultOutcome({ is_error: true, stop_reason: null, errors: ["context deadline exceeded"] }),
    ).toEqual({
      isError: true,
      stopReason: "stop",
      errorMessage: "context deadline exceeded",
    });

    expect(resultOutcome({ is_error: true, stop_reason: null, result: "The model refused to respond." })).toEqual({
      isError: true,
      stopReason: "stop",
      errorMessage: "The model refused to respond.",
    });
  });

  test("treats terminal_reason tool_deferred_unavailable as an error even when is_error is false", () => {
    expect(
      resultOutcome({ is_error: false, stop_reason: null, terminal_reason: "tool_deferred_unavailable" }),
    ).toEqual({
      isError: true,
      stopReason: "stop",
      errorMessage: "Claude Agent SDK could not honor the deferred Pi tool call (terminal_reason: tool_deferred_unavailable)",
    });
  });

  test("builds an honest Pi system prompt and a catalog for the deferred tool gateway", () => {
    const context = {
      systemPrompt: "Repository rule: run tests.",
      messages: [{ role: "user", content: "Inspect package.json" }],
      tools: [
        {
          name: "read",
          description: "Read a file",
          parameters: {
            type: "object",
            properties: { path: { type: "string" } },
            required: ["path"],
          },
        },
      ],
    } as unknown as Context;

    const request = buildAgentRequest(context);

    expect(request.systemPrompt).toContain("You are the model inside Pi Coding Agent");
    expect(request.systemPrompt).toContain("Repository rule: run tests.");
    expect(request.systemPrompt).not.toContain("You are Claude Code");
    expect(request.promptBlocks.map((block) => block.text)).toContain(
      '{"role":"user","content":[{"type":"text","text":"Inspect package.json"}]}',
    );
    expect(request.toolDescription).toContain('\"name\":\"read\"');
    expect(request.toolDescription).toContain('\"required\":[\"path\"]');
  });

  test("marks only the last conversation entry as a cache breakpoint on short transcripts, never the trailing instruction block", () => {
    const context = {
      systemPrompt: "s",
      messages: [
        { role: "user", content: "Read package.json" },
        {
          role: "assistant",
          content: [{ type: "toolCall", id: "call-1", name: "read", arguments: { path: "package.json" } }],
        },
      ],
      tools: [],
    } as unknown as Context;

    const request = buildAgentRequest(context);

    // [intro, entry0, entry1, outro]
    expect(request.promptBlocks).toHaveLength(4);
    expect(request.promptBlocks[0]?.cacheBreakpoint).toBeFalsy();
    expect(request.promptBlocks[1]?.cacheBreakpoint).toBeFalsy();
    expect(request.promptBlocks[2]?.cacheBreakpoint).toBe(true);
    expect(request.promptBlocks[3]?.cacheBreakpoint).toBeFalsy();
  });

  test("keeps one provider cache breakpoint on the transcript tail even after the transcript grows past 40 entries", () => {
    const contextWithEntries = (count: number) =>
      ({
        systemPrompt: "s",
        messages: Array.from({ length: count }, (_, index) => ({ role: "user", content: `entry ${index}` })),
        tools: [],
      }) as unknown as Context;
    const breakpointIndexes = (count: number) =>
      buildAgentRequest(contextWithEntries(count)).promptBlocks.flatMap((block, index) =>
        block.cacheBreakpoint ? [index] : [],
      );

    // promptBlocks = [intro, entry 0..N-1 (entry index + 1), outro].
    expect(breakpointIndexes(40)).toEqual([40]);
    expect(breakpointIndexes(41)).toEqual([41]);
    expect(breakpointIndexes(62)).toEqual([62]);
  });

  test("keeps the stable transcript prefix byte-identical as new entries are appended, so a later turn can hit cache on it", () => {
    const baseContext = {
      systemPrompt: "s",
      messages: [
        { role: "user", content: "Read package.json" },
        {
          role: "assistant",
          content: [{ type: "toolCall", id: "call-1", name: "read", arguments: { path: "package.json" } }],
        },
      ],
      tools: [],
    } as unknown as Context;
    const grownContext = {
      ...baseContext,
      messages: [
        ...baseContext.messages,
        {
          role: "toolResult",
          toolCallId: "call-1",
          toolName: "read",
          isError: false,
          content: [{ type: "text", text: "{}" }],
        },
      ],
    } as unknown as Context;

    const before = buildAgentRequest(baseContext);
    const after = buildAgentRequest(grownContext);

    // `before` is [intro, entry0, entry1, outro]; everything except the trailing
    // instruction block is the stable prefix a later turn should reproduce exactly.
    const stablePrefix = before.promptBlocks.slice(0, -1).map((block) => block.text);
    const afterPrefix = after.promptBlocks.slice(0, stablePrefix.length).map((block) => block.text);
    expect(afterPrefix).toEqual(stablePrefix);

    // The breakpoint moves forward onto the newest entry each turn: `before` marks
    // the last block of its stable prefix, but `after` marks the next block along
    // (its own newest entry) instead, now that the prefix is no longer the newest.
    expect(before.promptBlocks[stablePrefix.length - 1]?.cacheBreakpoint).toBe(true);
    expect(after.promptBlocks[stablePrefix.length - 1]?.cacheBreakpoint).toBeFalsy();
    expect(after.promptBlocks[stablePrefix.length]?.cacheBreakpoint).toBe(true);
  });
  test("declares image input support on every model so Pi's read tool attaches images instead of omitting them", () => {
    expect(models.length).toBeGreaterThan(0);
    for (const model of models) {
      expect(model.input).toEqual(["text", "image"]);
    }
  });

  test("forwards a user message's image bytes to the SDK as a real content block, not embedded in the JSONL text", () => {
    const context = {
      systemPrompt: "s",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this screenshot?" },
            { type: "image", data: "dXNlci1pbWFnZQ==", mimeType: "image/png" },
          ],
        },
      ],
      tools: [],
    } as unknown as Context;

    const request = buildAgentRequest(context);
    const entryBlock = request.promptBlocks.find((block) => block.text.includes('"role":"user"'));

    expect(entryBlock).toBeDefined();
    expect(entryBlock?.text).not.toContain("dXNlci1pbWFnZQ==");
    expect(entryBlock?.text).toBe(
      '{"role":"user","content":[{"type":"text","text":"What is in this screenshot?"},{"type":"image","mediaType":"image/png","imageRef":0}]}',
    );
    expect(entryBlock?.images).toEqual([{ data: "dXNlci1pbWFnZQ==", mediaType: "image/png" }]);
  });

  test("forwards a toolResult message's image bytes (e.g. a screenshot tool) the same way as user images", () => {
    const context = {
      systemPrompt: "s",
      messages: [
        { role: "user", content: "Take a screenshot" },
        {
          role: "toolResult",
          toolCallId: "call-1",
          toolName: "screenshot",
          isError: false,
          content: [{ type: "image", data: "dG9vbC1pbWFnZQ==", mimeType: "image/jpeg" }],
        },
      ],
      tools: [],
    } as unknown as Context;

    const request = buildAgentRequest(context);
    const entryBlock = request.promptBlocks.find((block) => block.text.includes('"role":"toolResult"'));

    expect(entryBlock).toBeDefined();
    expect(entryBlock?.text).not.toContain("dG9vbC1pbWFnZQ==");
    expect(entryBlock?.text).toBe(
      '{"role":"toolResult","toolCallId":"call-1","toolName":"screenshot","isError":false,"content":[{"type":"image","mediaType":"image/jpeg","imageRef":0}]}',
    );
    expect(entryBlock?.images).toEqual([{ data: "dG9vbC1pbWFnZQ==", mediaType: "image/jpeg" }]);
  });

  test("keeps the stable transcript prefix (text and images) byte-identical across turns when images are present", () => {
    const baseContext = {
      systemPrompt: "s",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "Look at this" },
            { type: "image", data: "aW1hZ2Utb25l", mimeType: "image/png" },
          ],
        },
        {
          role: "assistant",
          content: [{ type: "toolCall", id: "call-1", name: "read", arguments: { path: "package.json" } }],
        },
      ],
      tools: [],
    } as unknown as Context;
    const grownContext = {
      ...baseContext,
      messages: [
        ...baseContext.messages,
        {
          role: "toolResult",
          toolCallId: "call-1",
          toolName: "read",
          isError: false,
          content: [{ type: "text", text: "{}" }],
        },
      ],
    } as unknown as Context;

    const before = buildAgentRequest(baseContext);
    const after = buildAgentRequest(grownContext);

    const stablePrefix = before.promptBlocks.slice(0, -1);
    const afterPrefix = after.promptBlocks.slice(0, stablePrefix.length);
    expect(afterPrefix.map((block) => block.text)).toEqual(stablePrefix.map((block) => block.text));
    expect(afterPrefix.map((block) => block.images)).toEqual(stablePrefix.map((block) => block.images));

    const imageBlock = stablePrefix.find((block) => (block.images?.length ?? 0) > 0);
    expect(imageBlock?.images).toEqual([{ data: "aW1hZ2Utb25l", mediaType: "image/png" }]);
  });
});

describe("unsupported image mime types", () => {
  test("an unsupported image kept in transcript history does not throw on a later turn", async () => {
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;
    const runSdkQuery: RunSdkQuery = (() => (async function* () {
      yield { type: "result", is_error: false, stop_reason: "end_turn" };
    })()) as RunSdkQuery;
    const runner = createClaudeAgentSdkRunner(runSdkQuery);

    const context = {
      systemPrompt: "s",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What format is this?" },
            { type: "image", data: "dW5zdXBwb3J0ZWQ=", mimeType: "image/bmp" },
          ],
        },
      ],
      tools: [],
    } as unknown as Context;

    // Turn 1: the unsupported image is the newest (and only) entry.
    const firstRequest = buildAgentRequest(context);
    const firstTurnEvents: BridgeEvent[] = [];
    for await (const event of runner(firstRequest, model)) firstTurnEvents.push(event);
    expect(firstTurnEvents).toEqual([{ type: "done", reason: "stop" }]);

    // Turn 2: the same message is now historical, re-serialized and resent
    // unchanged, alongside a new user message — this must not throw.
    const grownContext = {
      ...context,
      messages: [...context.messages, { role: "user", content: "Never mind, thanks" }],
    } as unknown as Context;
    const secondRequest = buildAgentRequest(grownContext);
    const secondTurnEvents: BridgeEvent[] = [];
    for await (const event of runner(secondRequest, model)) secondTurnEvents.push(event);
    expect(secondTurnEvents).toEqual([{ type: "done", reason: "stop" }]);
  });
});

describe("buildPromptStream", () => {
  test("leaves room for the SDK's three cache breakpoints so the API never receives five", async () => {
    // The sanitized failing session had 41 ancestor messages at its first
    // error, which was the old threshold for adding a second provider marker.
    const request = buildAgentRequest({
      systemPrompt: "s",
      messages: Array.from({ length: 41 }, (_, index) => ({ role: "user", content: `entry ${index}` })),
      tools: [],
    } as unknown as Context);
    const model = {
      api: "claude-sdk",
      provider: "claude-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-sdk">;
    const runSdkQuery: RunSdkQuery = async function* ({ prompt }) {
      const [message] = await drain(prompt as AsyncIterable<SDKUserMessage>);
      const providerBreakpoints = (message?.message.content as unknown as Array<Record<string, unknown>>).filter(
        (block) => block.cache_control !== undefined,
      ).length;
      const cacheControlBlocksAtApi = 3 + providerBreakpoints;
      if (cacheControlBlocksAtApi > 4) {
        throw new Error(
          `API Error: 400 A maximum of 4 blocks with cache_control may be provided. Found ${cacheControlBlocksAtApi}.`,
        );
      }
      yield { type: "result", is_error: false, stop_reason: "end_turn" };
    };

    const events = await drain(createClaudeAgentSdkRunner(runSdkQuery)(request, model));

    expect(events).toEqual([{ type: "done", reason: "stop" }]);
  });

  test("keeps only the latest requested cache breakpoint when given multiple marked prompt blocks", async () => {
    const messages = await drain(
      buildPromptStream([
        { text: "older prefix", cacheBreakpoint: true },
        { text: "newer prefix", cacheBreakpoint: true },
      ]),
    );

    expect(messages[0]?.message.content).toEqual([
      { type: "text", text: "older prefix" },
      { type: "text", text: "newer prefix", cache_control: { type: "ephemeral", ttl: "1h" } },
    ]);
  });

  test("sends the whole transcript as one SDK user message with one content block per prompt block", async () => {
    const messages = await drain(
      buildPromptStream([
        { text: "intro" },
        { text: "entry-0" },
        { text: "entry-1", cacheBreakpoint: true },
        { text: "outro" },
      ]),
    );

    expect(messages).toHaveLength(1);
    const [message] = messages;
    expect(message?.type).toBe("user");
    expect(message?.message.content).toEqual([
      { type: "text", text: "intro" },
      { type: "text", text: "entry-0" },
      { type: "text", text: "entry-1", cache_control: { type: "ephemeral", ttl: "1h" } },
      { type: "text", text: "outro" },
    ]);
  });

  test("expands a block's images into real Anthropic base64 image blocks right after its text block", async () => {
    const messages = await drain(
      buildPromptStream([
        { text: "intro" },
        {
          text: '{"role":"user","content":[...]}',
          images: [{ data: "aW1hZ2UtZGF0YQ==", mediaType: "image/png" }],
        },
      ]),
    );

    const [message] = messages;
    expect(message?.message.content).toEqual([
      { type: "text", text: "intro" },
      { type: "text", text: '{"role":"user","content":[...]}' },
      {
        type: "image",
        source: { type: "base64", media_type: "image/png", data: "aW1hZ2UtZGF0YQ==" },
      },
    ]);
  });

  test("puts the cache breakpoint on the last image, not the text block, when a cached entry carries images", async () => {
    const messages = await drain(
      buildPromptStream([
        {
          text: '{"role":"user","content":[...]}',
          cacheBreakpoint: true,
          images: [
            { data: "Zmlyc3Q=", mediaType: "image/png" },
            { data: "c2Vjb25k", mediaType: "image/jpeg" },
          ],
        },
      ]),
    );

    const [message] = messages;
    const content = message?.message.content as unknown as Array<Record<string, unknown>>;
    expect(content).toHaveLength(3);
    expect(content[0]).not.toHaveProperty("cache_control");
    expect(content[1]).not.toHaveProperty("cache_control");
    expect(content[2]).toMatchObject({
      type: "image",
      source: { type: "base64", media_type: "image/jpeg", data: "c2Vjb25k" },
      cache_control: { type: "ephemeral", ttl: "1h" },
    });
  });

  test("degrades an unsupported image mime type to a text note instead of throwing", async () => {
    const messages = await drain(
      buildPromptStream([{ text: "t", images: [{ data: "ZGF0YQ==", mediaType: "image/bmp" }] }]),
    );

    const [message] = messages;
    expect(message?.message.content).toEqual([
      { type: "text", text: "t" },
      { type: "text", text: '[Image data omitted from transcript: unsupported mime type "image/bmp"]' },
    ]);
  });

  test("keeps supported images as real image blocks alongside a degraded unsupported one in the same message", async () => {
    const messages = await drain(
      buildPromptStream([
        {
          text: "t",
          images: [
            { data: "c3VwcG9ydGVk", mediaType: "image/png" },
            { data: "dW5zdXBwb3J0ZWQ=", mediaType: "image/bmp" },
          ],
        },
      ]),
    );

    const [message] = messages;
    expect(message?.message.content).toEqual([
      { type: "text", text: "t" },
      {
        type: "image",
        source: { type: "base64", media_type: "image/png", data: "c3VwcG9ydGVk" },
      },
      { type: "text", text: '[Image data omitted from transcript: unsupported mime type "image/bmp"]' },
    ]);
  });

  test("puts the cache breakpoint on the degraded text note when the last image of a cached entry is unsupported", async () => {
    const messages = await drain(
      buildPromptStream([
        {
          text: '{"role":"user","content":[...]}',
          cacheBreakpoint: true,
          images: [
            { data: "c3VwcG9ydGVk", mediaType: "image/png" },
            { data: "dW5zdXBwb3J0ZWQ=", mediaType: "image/bmp" },
          ],
        },
      ]),
    );

    const [message] = messages;
    const content = message?.message.content as unknown as Array<Record<string, unknown>>;
    expect(content).toHaveLength(3);
    expect(content[0]).not.toHaveProperty("cache_control");
    expect(content[1]).not.toHaveProperty("cache_control");
    expect(content[2]).toMatchObject({
      type: "text",
      text: '[Image data omitted from transcript: unsupported mime type "image/bmp"]',
      cache_control: { type: "ephemeral", ttl: "1h" },
    });
  });

  test("produces the same degraded blocks turn after turn instead of throwing on a historical entry", async () => {
    const promptBlocks = [
      { text: "intro" },
      {
        text: '{"role":"user","content":[...]}',
        images: [{ data: "ZGF0YQ==", mediaType: "image/bmp" }],
      },
    ];

    const firstTurn = await drain(buildPromptStream(promptBlocks));
    const secondTurn = await drain(buildPromptStream(promptBlocks));

    expect(secondTurn[0]?.message.content).toEqual(firstTurn[0]?.message.content);
    expect(secondTurn[0]?.message.content).toEqual([
      { type: "text", text: "intro" },
      { type: "text", text: '{"role":"user","content":[...]}' },
      { type: "text", text: '[Image data omitted from transcript: unsupported mime type "image/bmp"]' },
    ]);
  });
});
