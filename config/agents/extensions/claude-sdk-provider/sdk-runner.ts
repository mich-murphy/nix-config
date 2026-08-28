// Compatibility facade for existing imports. The implementation is split by
// reason to change under ./sdk so protocol, prompt, tool, environment, and
// orchestration concerns can evolve independently.
export {
  createDeferredPiCallHandler,
  createPreToolUseHook,
  type DeferredCall,
} from "./sdk/deferred-tools";
export {
  resultOutcome,
  translateSdkStreamEvent,
  type ResultOutcome,
} from "./sdk/event-translation";
export { buildPromptStream } from "./sdk/prompt-stream";
export {
  agentSdkTurnOptions,
  createClaudeAgentSdkRunner,
  runClaudeAgentSdk,
  type RunSdkQuery,
} from "./sdk/runner";
export { subscriptionEnvironment } from "./sdk/subscription-environment";
