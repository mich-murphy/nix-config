import {
  isToolCallEventType,
  type ExtensionAPI,
  type ProviderModelConfig,
} from "@earendil-works/pi-coding-agent";
import { createAgentSdkStream } from "./bridge";
import { cacheDiagnosticsFromEnvironment } from "./cache-diagnostics";
import { inspectBashCommand, sanitizeBashContent, sanitizeContextMessages } from "./output-safety";
import { createClaudeAgentSdkRunner } from "./sdk-runner";

/** Models exposed by the official Claude Agent SDK provider. */
export const models: ReadonlyArray<ProviderModelConfig> = [
  { id: "sonnet", name: "Claude Sonnet (official Agent SDK)", reasoning: true },
  { id: "opus", name: "Claude Opus (official Agent SDK)", reasoning: true },
  { id: "fable", name: "Claude Fable (official Agent SDK)", reasoning: true },
  { id: "haiku", name: "Claude Haiku (official Agent SDK)", reasoning: false },
].map<ProviderModelConfig>(({ id, name, reasoning }) => ({
  id,
  name,
  reasoning,
  input: ["text", "image"],
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  contextWindow: 200_000,
  maxTokens: 64_000,
}));

/**
 * Register the Claude Agent SDK provider and bash-output safety hooks.
 *
 * @param pi - Pi extension API supplied at startup.
 */
export default function registerClaudeSdkProvider(pi: ExtensionAPI): void {
  const runClaudeAgentSdk = createClaudeAgentSdkRunner(undefined, cacheDiagnosticsFromEnvironment());

  pi.on("before_agent_start", (event) => ({
    systemPrompt: `${event.systemPrompt}\n\nBash output safety: never cat an executable or print raw binary/base64 data. Use file, otool, or strings for executables, and inspect encoded files via metadata instead of stdout.`,
  }));

  pi.on("tool_call", (event) => {
    if (!isToolCallEventType("bash", event)) return;
    const reason = inspectBashCommand(event.input.command);
    if (reason) return { block: true, reason };
  });

  pi.on("tool_result", (event, ctx) => {
    if (event.toolName !== "bash") return;
    const sanitized = sanitizeBashContent(event.content);
    if (!sanitized.detected) return;
    // Headless sub-agents (-p print mode) have no UI surface; the quarantine
    // notice is already embedded in the sanitized tool-result content, so the
    // model still sees it. Only the human-facing toast needs a UI.
    if (ctx.hasUI) {
      ctx.ui.notify(
        `Quarantined ${sanitized.detected}-like bash output before it entered context. Compact or start a new session if similar output was recorded earlier.`,
        "warning",
      );
    }
    return { content: sanitized.content };
  });

  pi.on("context", (event) => ({ messages: sanitizeContextMessages(event.messages) }));

  pi.registerProvider("claude-sdk", {
    name: "Claude subscription via official Agent SDK",
    baseUrl: "agent-sdk://local-claude-code",
    apiKey: "claude-sdk-managed-auth",
    api: "claude-sdk",
    models: [...models],
    streamSimple: (model, context, options) =>
      createAgentSdkStream(model, context, options, runClaudeAgentSdk),
  });
}
