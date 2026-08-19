import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { createAgentSdkStream } from "./bridge";
import { createClaudeAgentSdkRunner } from "./sdk-runner";

export const models = [
  { id: "sonnet", name: "Claude Sonnet (official Agent SDK)" },
  { id: "opus", name: "Claude Opus (official Agent SDK)" },
  { id: "fable", name: "Claude Fable (official Agent SDK)" },
].map(({ id, name }) => ({
  id,
  name,
  reasoning: true,
  input: ["text", "image"] as ["text", "image"],
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  contextWindow: 200_000,
  maxTokens: 64_000,
}));

export default function (pi: ExtensionAPI) {
  const runClaudeAgentSdk = createClaudeAgentSdkRunner();

  pi.registerProvider("claude-sdk", {
    name: "Claude subscription via official Agent SDK",
    baseUrl: "agent-sdk://local-claude-code",
    apiKey: "claude-sdk-managed-auth",
    api: "claude-sdk",
    models,
    streamSimple: (model, context, options) =>
      createAgentSdkStream(model, context, options, runClaudeAgentSdk),
  });
}
