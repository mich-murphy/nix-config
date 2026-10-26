{
  config,
  lib,
  pkgs,
  repoRoot,
  ...
}: let
  # eMudhra intermediate CA compromise (CVE-2026-111943): Bun embeds its own
  # root store, and bun 1.4.2 still trusts the compromised emSign intermediates
  # (verified against dv-g3b-emudhra.we-have-a-problem.com; --use-system-ca does
  # not help). Bun 1.4.4 is patched. Pin it until nixpkgs ships it
  # (NixOS/nixpkgs#577354); the version guard makes this a no-op afterwards.
  bunPatched =
    if lib.versionOlder pkgs.bun.version "1.4.4"
    then
      pkgs.bun.overrideAttrs (old: {
        version = "1.4.4";
        passthru =
          old.passthru
          // {
            sources = {
              "aarch64-darwin" = pkgs.fetchurl {
                url = "https://github.com/oven-sh/bun/releases/download/bun-v1.4.4/bun-darwin-aarch64.zip";
                hash = "sha256-wkDX8My5DCY6hJ/NsJt+5iYw2SE0jtaq/ahrnVd7xRA=";
              };
              "aarch64-linux" = pkgs.fetchurl {
                url = "https://github.com/oven-sh/bun/releases/download/bun-v1.4.4/bun-linux-aarch64.zip";
                hash = "sha256-uqa7NvUdblhWHgK76oJQOKnfRU9nPW13bou/hvjHhQ4=";
              };
              "x86_64-linux" = pkgs.fetchurl {
                url = "https://github.com/oven-sh/bun/releases/download/bun-v1.4.4/bun-linux-x64-baseline.zip";
                hash = "sha256-4ERutVmVdy65iUvrrppr42jZhai5Jx7hJEhvmUbu3XY=";
              };
            };
          };
      })
    else pkgs.bun;
  agentConfig = "${repoRoot}/config/agents";
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in {
  home.packages = [
    # The harness hooks (~/dev/harness/skills/mm-mode/hooks) run on Bun.
    bunPatched
  ];
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/CLAUDE.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".codex/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".pi/agent/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    # Per-skill links keep ~/.claude/skills a real directory so Claude
    # Code's managed-skills sync cannot write into the repo.
    ".claude/skills/bro".source = liveLink "${agentConfig}/skills/bro";
    ".claude/skills/create-verification".source =
      liveLink "${agentConfig}/skills/create-verification";
    ".claude/skills/show-me".source = liveLink "${agentConfig}/skills/show-me";
  };
}
