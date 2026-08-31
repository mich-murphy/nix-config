{
  config,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  agentConfigSources = ../config/agents;
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in {
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/skills".source = liveLink "${agentConfig}/skills";
    ".pi/agent/agents/Explore.md".source = agentConfigSources + "/agents/Explore.md";
    ".pi/agent/agents/Research.md".source = agentConfigSources + "/agents/Research.md";
  };
}
