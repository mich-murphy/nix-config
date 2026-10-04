{
  config,
  lib,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;

  # Shell-launched Claude Code goes through the CLIProxyAPI proxy on ai-dev.
  # The proxy has no client keys; the placeholder token only stops Claude
  # Code sending the real OAuth token. GUI launches outside a shell stay direct.
  claudeProxyEnv = {
    ANTHROPIC_BASE_URL = "https://ai-dev.zonkey-goblin.ts.net";
    ANTHROPIC_AUTH_TOKEN = "cliproxy-placeholder-not-a-secret";
    CLAUDE_CODE_PROMPT_CACHE_TTL = "1h";
    CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL = "1h";
  };
in {
  home.sessionVariables = claudeProxyEnv;

  # Direct fallbacks that bypass the proxy for a single invocation.
  programs.fish.functions = {
    claude-direct = {
      description = "Run Claude Code directly, bypassing CLIProxyAPI";
      wraps = "claude";
      body = ''
        env ${lib.concatMapStringsSep " " (name: "-u ${name}") (lib.attrNames claudeProxyEnv)} claude $argv
      '';
    };
    codex-direct = {
      description = "Run Codex directly, bypassing CLIProxyAPI";
      wraps = "codex";
      body = ''
        command codex -c model_provider=openai $argv
      '';
    };
  };

  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    # Per-skill links keep ~/.claude/skills a real directory so Claude
    # Code's managed-skills sync cannot write into the repo.
    ".claude/skills/bro".source = liveLink "${agentConfig}/skills/bro";
    ".claude/skills/herdr".source = liveLink "${agentConfig}/skills/herdr";
    ".claude/skills/no-mistakes".source =
      liveLink "${agentConfig}/skills/no-mistakes";
    ".claude/skills/research".source = liveLink "${agentConfig}/skills/research";
    ".claude/skills/typescript".source =
      liveLink "${agentConfig}/skills/typescript";
    ".claude/skills/unslop".source = liveLink "${agentConfig}/skills/unslop";
    ".codex/skills/no-mistakes".source =
      liveLink "${agentConfig}/skills/no-mistakes";
  };
}
