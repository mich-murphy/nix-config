{
  config,
  pkgs,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in {
  home.packages = [
    # The harness hooks (~/dev/harness/skills/mm-mode/hooks) run on Bun.
    pkgs.bun
  ];
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    # Per-skill links keep ~/.claude/skills a real directory so Claude
    # Code's managed-skills sync cannot write into the repo.
    ".claude/skills/bro".source = liveLink "${agentConfig}/skills/bro";
    ".claude/skills/create-verification".source =
      liveLink "${agentConfig}/skills/create-verification";
    ".claude/skills/show-me".source = liveLink "${agentConfig}/skills/show-me";
  };
}
