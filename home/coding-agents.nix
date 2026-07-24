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
  };
}
