{
  config,
  pkgs,
  repoRoot,
  agent-sessions,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in {
  home.packages = [
    # The harness hooks (~/dev/harness/skills/mm-mode/hooks) run on Bun.
    pkgs.bun
    # Session collector. Its hooks run the profile path, which survives upgrades, so
    # Codex's hook trust stays valid; run `agent-sessions install` once after the first switch.
    agent-sessions.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    # Per-skill links keep ~/.claude/skills a real directory so Claude
    # Code's managed-skills sync cannot write into the repo.
    ".claude/skills/bro".source = liveLink "${agentConfig}/skills/bro";
    ".claude/skills/herdr".source = liveLink "${agentConfig}/skills/herdr";
    ".claude/skills/no-mistakes".source =
      liveLink "${agentConfig}/skills/no-mistakes";
    ".claude/skills/research".source = liveLink "${agentConfig}/skills/research";
    ".claude/skills/show-me".source = liveLink "${agentConfig}/skills/show-me";
    ".claude/skills/typescript".source =
      liveLink "${agentConfig}/skills/typescript";
    ".claude/skills/unslop".source = liveLink "${agentConfig}/skills/unslop";
    ".codex/skills/no-mistakes".source =
      liveLink "${agentConfig}/skills/no-mistakes";
  };
}
