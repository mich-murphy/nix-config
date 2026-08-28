// The API beta required for ttl: "1h" cache-control blocks.
const EXTENDED_CACHE_TTL_BETA = "extended-cache-ttl-2025-04-11";

const NON_SUBSCRIPTION_AUTH_VARIABLES = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "CLAUDE_CODE_USE_BEDROCK",
  "CLAUDE_CODE_USE_VERTEX",
  "CLAUDE_CODE_USE_FOUNDRY",
] as const;

export function subscriptionEnvironment(
  source: Record<string, string | undefined> = process.env,
): Record<string, string | undefined> {
  const environment = { ...source };
  for (const name of NON_SUBSCRIPTION_AUTH_VARIABLES) delete environment[name];
  environment.CLAUDE_AGENT_SDK_CLIENT_APP = "pi-coding-agent-provider/0.1.0";

  // Pin the extended TTL because Claude Code otherwise gates its own 1h choice
  // independently of the cache_control block supplied by this provider.
  // PI_CLAUDE_SDK_5M_CACHE restores the CLI's native policy as an escape hatch.
  if (environment.PI_CLAUDE_SDK_5M_CACHE === "1") return environment;
  delete environment.FORCE_PROMPT_CACHING_5M;
  environment.ENABLE_PROMPT_CACHING_1H = "1";
  const betas = (environment.ANTHROPIC_BETAS ?? "")
    .split(",")
    .map((beta) => beta.trim())
    .filter(Boolean);
  if (!betas.includes(EXTENDED_CACHE_TTL_BETA)) betas.push(EXTENDED_CACHE_TTL_BETA);
  environment.ANTHROPIC_BETAS = betas.join(",");
  return environment;
}
