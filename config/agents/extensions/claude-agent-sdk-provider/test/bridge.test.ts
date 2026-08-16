import { describe, expect, test } from "bun:test";
import type { Context, Model } from "@earendil-works/pi-ai";
import { buildAgentRequest, createAgentSdkStream, serializeConversation, type AgentRequest, type BridgeEvent } from "../bridge";
import {
  agentSdkTurnOptions,
  createClaudeAgentSdkRunner,
  createDeferredPiCallHandler,
  subscriptionEnvironment,
  translateSdkStreamEvent,
  type RunSdkQuery,
} from "../sdk-runner";

describe("serializeConversation", () => {
  test("starts each Pi turn in a stateless SDK session with the complete transcript", async () => {
    const sdkRequests: Array<Parameters<RunSdkQuery>[0]> = [];
    const runSdkQuery = (params: (typeof sdkRequests)[number]) => {
      sdkRequests.push(params);
      return (async function* () {})();
    };
    const runner = createClaudeAgentSdkRunner(runSdkQuery);
    const model = {
      api: "claude-agent-sdk",
      provider: "claude-agent-sdk",
      id: "sonnet",
    } as unknown as Model<"claude-agent-sdk">;
    const firstRequest: AgentRequest = {
      systemPrompt: "stable system prompt",
      prompt: "complete transcript: first turn",
      toolDescription: "stable tools",
      toolNames: ["read"],
      conversationEntries: ['{"role":"user","content":"Read package.json"}'],
    };
    const secondRequest: AgentRequest = {
      ...firstRequest,
      prompt: "complete transcript: second turn",
      conversationEntries: [
        ...firstRequest.conversationEntries,
        '{"role":"assistant","content":[{"type":"toolCall","id":"call-1","name":"read","arguments":{"path":"package.json"}}]}',
        '{"role":"toolResult","toolCallId":"call-1","toolName":"read","isError":false,"content":[{"type":"text","text":"{}"}]}',
      ],
    };

    for await (const _event of runner(firstRequest, model)) {}
    for await (const _event of runner(secondRequest, model)) {}

    expect(sdkRequests.map(({ prompt }) => prompt)).toEqual([firstRequest.prompt, secondRequest.prompt]);
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
      api: "claude-agent-sdk",
      provider: "claude-agent-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-agent-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "text_delta", text: "Hello" };
      yield { type: "text_delta", text: " from Claude" };
      yield { type: "usage", input: 12, output: 3, cacheRead: 4, cacheWrite: 0 };
      yield { type: "done" };
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
      expect(done.message.content).toEqual([{ type: "text", text: "Hello from Claude" }]);
      expect(done.message.usage).toMatchObject({ input: 12, output: 3, cacheRead: 4, totalTokens: 19 });
    }
  });

  test("ends the Pi turn with a deferred tool call from the SDK gateway", async () => {
    const context = {
      systemPrompt: "Use tools when needed.",
      messages: [{ role: "user", content: "Read package.json" }],
      tools: [],
    } as unknown as Context;
    const model = {
      api: "claude-agent-sdk",
      provider: "claude-agent-sdk",
      id: "sonnet",
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    } as unknown as Model<"claude-agent-sdk">;
    const run = async function* (): AsyncGenerator<BridgeEvent> {
      yield { type: "tool_call", id: "tool-1", name: "read", arguments: { path: "package.json" } };
      yield { type: "done" };
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

  test("lets Pi own the multi-turn tool loop instead of capping each SDK query at one model turn", () => {
    expect(agentSdkTurnOptions()).toEqual({});
  });

  test("captures a Pi tool request when the SDK executes the MCP gateway directly", async () => {
    let deferred: Extract<BridgeEvent, { type: "tool_call" }> | undefined;
    const handler = createDeferredPiCallHandler(
      new Set(["read"]),
      (toolCall) => {
        deferred = toolCall;
      },
      () => {},
      () => "fallback-tool-id",
    );

    const result = await handler({ name: "read", arguments: { path: "package.json" } });

    expect(deferred).toEqual({
      type: "tool_call",
      id: "fallback-tool-id",
      name: "read",
      arguments: { path: "package.json" },
    });
    expect(result).toEqual({
      content: [{ type: "text", text: "Tool execution is deferred to Pi." }],
    });
  });

  test("rejects an unknown Pi tool requested through the direct MCP gateway", async () => {
    let deferred: Extract<BridgeEvent, { type: "tool_call" }> | undefined;
    let capturedError: Error | undefined;
    const handler = createDeferredPiCallHandler(
      new Set(["read"]),
      (toolCall) => {
        deferred = toolCall;
      },
      (error) => {
        capturedError = error;
      },
    );

    const result = await handler({ name: "missing_tool", arguments: {} });

    expect(deferred).toBeUndefined();
    expect(capturedError?.message).toBe("Claude requested an invalid Pi tool call: missing_tool");
    expect(result).toEqual({
      content: [{ type: "text", text: "Claude requested an invalid Pi tool call: missing_tool" }],
      isError: true,
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
    expect(request.prompt).toContain('{"role":"user","content":[{"type":"text","text":"Inspect package.json"}]}');
    expect(request.toolDescription).toContain('\"name\":\"read\"');
    expect(request.toolDescription).toContain('\"required\":[\"path\"]');
  });
});
