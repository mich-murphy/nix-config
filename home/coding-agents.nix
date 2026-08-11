{
  config,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in {
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/skills".source = liveLink "${agentConfig}/skills";
    ".pi/agent/extensions" = {
      source = liveLink "${agentConfig}/extensions";
      force = true;
    };
  };
}
