{
  config,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in {
  home.file = {
    ".claude/CLAUDE.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".codex/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".pi/agent/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/skills".source = liveLink "${agentConfig}/skills";
    ".codex/observability.config.toml".source = liveLink "${agentConfig}/telemetry/codex-observability.config.toml";
    ".claude/observability.settings.json".source = liveLink "${agentConfig}/telemetry/claude-observability.settings.json";
    ".pi/agent/extensions/app-agent-otel.ts".source = liveLink "${agentConfig}/telemetry/pi/app-agent-otel.ts";
    ".config/agent-observability".source = liveLink "${agentConfig}/telemetry";
  };

  programs.fish.shellAliases = {
    codex = "command codex --profile observability";
    claude = "command claude --settings ~/.claude/observability.settings.json";
  };
}
