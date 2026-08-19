# Claude subscription provider for Pi

This Pi extension routes model turns through Anthropic's official Claude Agent SDK. Claude Code owns authentication; the extension does not read, copy, replay, or spoof OAuth tokens and does not call private Anthropic endpoints.

## Prerequisites

1. Install Claude Code and authenticate with the subscription account:

   ```sh
   claude auth login
   claude auth status --text
   ```

2. Install this extension's pinned dependencies:

   ```sh
   npm install --ignore-scripts --prefix ~/.pi/agent/extensions/claude-sdk-provider
   ```

3. Restart Pi or run `/reload`.

## Use

Open `/model` and select one of:

- `claude-sdk/sonnet`
- `claude-sdk/opus`
- `claude-sdk/fable`

The provider deliberately removes API-key and Bedrock, Vertex, and Foundry routing variables from the Agent SDK subprocess. This keeps the provider on Claude's first-party subscription authentication instead of silently falling back to separately billed API or cloud-provider usage.

## Architecture

- Pi remains the visible coding harness and owns its conversation, tools, approvals, and tool execution.
- Each Pi model turn is sent through the official `@anthropic-ai/claude-agent-sdk` `query()` API.
- The query does not set the SDK's `maxTurns`; Pi owns the outer tool loop and cancellation. A one-turn SDK cap can fail before a deferred tool request returns to Pi.
- The SDK receives a custom system prompt identifying Pi honestly; the extension does not impersonate Claude Code.
- Pi tools are advertised through one in-process MCP gateway tool, `pi_call`. A `PreToolUse` hook resolves every `pi_call` request with `permissionDecision: "defer"`, which ends the SDK's `query()` call cleanly (`terminal_reason: "tool_deferred"`) once every tool_use in that turn has been resolved — the SDK never executes the tool itself, and the caller does not need to race an `AbortController` against the SDK's own deny-handling to stop the turn. A request for any other tool name is denied outright (defense in depth; `tools: []` and `settingSources: []` should already make that path unreachable).
- The hook captures every deferred call it sees, so a turn in which the model batches several `pi_call` requests together (parallel tool use) hands all of them back to Pi, not just the first.
- The `pi_call` MCP tool's own `handler` is a defensive fallback only — the hook resolves permission before it can run in normal operation. If the SDK ever invokes it anyway (the hook's `defer` decision was not honored), it returns an error result and does not forward the call to Pi, instead of faking a successful defer.
- A `result` message that reports `is_error: true`, or `terminal_reason: "tool_deferred_unavailable"` (a requested defer the SDK could not honor), is surfaced as a real provider error instead of being treated as a clean stop — checked before any tool calls the hook already captured are handed to Pi, so a disagreeing SDK result always wins.
- Pi executes the tool(s) normally. The next model turn includes the resulting Pi transcript.
- SDK session persistence is disabled because Pi is the durable conversation owner.
- The transcript is sent as one content block per JSONL entry (via the SDK's streaming-input `prompt: AsyncIterable<SDKUserMessage>` mode, not the plain-string `prompt` path, which always collapses everything into a single block). Pi only ever appends to the transcript, so entries the previous turn already sent are byte-identical this turn; the last entry carries an explicit `cache_control: { type: "ephemeral" }` breakpoint (Anthropic's documented multi-turn caching pattern) so the API can serve that unchanged prefix from cache and pay only for the newly appended suffix, instead of rewriting the whole, ever-growing transcript to cache every turn.

## Current boundaries

- Text input only. Image data is not sent through this provider.
- Model IDs use Claude Code's documented moving aliases (`sonnet`, `opus`, and `fable`), so the underlying model can change when Anthropic updates an alias.
- Pi records subscription cost as zero. Token usage is retained when the SDK reports it, but Pi cannot infer the monetary value of an included subscription allocation.
- Reasoning/thinking deltas are streamed to Pi as a `thinking` content block, but the block is dropped (not replayed) when a later turn re-serializes the transcript — thinking is ephemeral, not part of the durable Pi conversation.
- An SDK `result` that ends in an error (`is_error: true`) is surfaced as a real provider error instead of a silent empty response; a `max_tokens` stop is reported to Pi as a `length` stop reason.

## Checks

```sh
npm test --prefix ~/.pi/agent/extensions/claude-sdk-provider
npm run check --prefix ~/.pi/agent/extensions/claude-sdk-provider
pi --list-models claude-sdk
```
