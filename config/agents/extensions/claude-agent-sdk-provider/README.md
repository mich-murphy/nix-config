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
   npm install --ignore-scripts --prefix ~/.pi/agent/extensions/claude-agent-sdk-provider
   ```

3. Restart Pi or run `/reload`.

## Use

Open `/model` and select one of:

- `claude-agent-sdk/sonnet`
- `claude-agent-sdk/opus`
- `claude-agent-sdk/fable`

The provider deliberately removes API-key and Bedrock, Vertex, and Foundry routing variables from the Agent SDK subprocess. This keeps the provider on Claude's first-party subscription authentication instead of silently falling back to separately billed API or cloud-provider usage.

## Architecture

- Pi remains the visible coding harness and owns its conversation, tools, approvals, and tool execution.
- Each Pi model turn is sent through the official `@anthropic-ai/claude-agent-sdk` `query()` API.
- The query does not set the SDK's `maxTurns`; Pi owns the outer tool loop and cancellation. A one-turn SDK cap can fail before a deferred tool request returns to Pi.
- The SDK receives a custom system prompt identifying Pi honestly; the extension does not impersonate Claude Code.
- Pi tools are advertised through one in-process MCP gateway tool. The SDK permission callback captures a requested Pi tool and returns it to Pi without executing it inside the SDK.
- Pi executes the tool normally. The next model turn includes the resulting Pi transcript.
- SDK session persistence is disabled because Pi is the durable conversation owner.

## Current boundaries

- Text input only. Image data is not sent through this provider.
- One deferred Pi tool call is requested per model turn. Pi can continue with further calls on subsequent turns.
- The complete Pi transcript is serialized into each SDK request. This favors correctness and stateless recovery over prompt-cache efficiency.
- Model IDs use Claude Code's documented moving aliases (`sonnet`, `opus`, and `fable`), so the underlying model can change when Anthropic updates an alias.
- Pi records subscription cost as zero. Token usage is retained when the SDK reports it, but Pi cannot infer the monetary value of an included subscription allocation.

## Checks

```sh
npm test --prefix ~/.pi/agent/extensions/claude-agent-sdk-provider
npm run check --prefix ~/.pi/agent/extensions/claude-agent-sdk-provider
pi --list-models claude-agent-sdk
```
