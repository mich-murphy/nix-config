{ config, ... }:
let
  agentConfig = "${config.nixConfig.checkout}/config/agents";
  skills = config.nixConfig.skills;
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in
{
  home.file = {
    ".agents/skills".source = liveLink skills;
    ".claude/CLAUDE.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".codex/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".pi/agent/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    # Per-skill links keep ~/.claude/skills a real directory so Claude
    # Code's managed-skills sync cannot write into the skills repo.
    ".claude/skills/bro".source = liveLink "${skills}/bro";
    ".claude/skills/coordinator".source = liveLink "${skills}/coordinator";
    ".claude/skills/create-verification".source = liveLink "${skills}/create-verification";
    ".claude/skills/no-comments".source = liveLink "${skills}/no-comments";
    ".claude/skills/show-me".source = liveLink "${skills}/show-me";
  };

  # Settings files stay regular, app-owned files: activation merges the keys
  # below into them and leaves everything else alone. Hooks (agent-sessions,
  # herdr, Moshi), model and effort choices, Pi's deviceId and changelog
  # versions stay local. Homebrew installs both applications.
  programs.claude-code = {
    enable = true;
    package = null;
    mutableSettings = true;
    settings = {
      autoCompactEnabled = true;
      autoContinueAtUsageLimit = false;
      autoMemoryEnabled = false;
      autoScrollEnabled = true;
      awaySummaryEnabled = false;
      cleanupPeriodDays = 3650;
      editorMode = "vim";
      enabledPlugins = {
        "frontend-design@claude-plugins-official" = false;
        "honeycomb@anthropic-plugin-directory" = true;
        "skill-creator@claude-plugins-official" = false;
      };
      env = {
        CLAUDE_CODE_PROMPT_CACHE_TTL = "1h";
        CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL = "1h";
      };
      feedbackDrafts = "off";
      permissions = {
        defaultMode = "auto";
        allow = [
          "Read"
          "Glob"
          "Grep"
          "Edit"
          "Write"
          "Bash(nix fmt)"
          "Bash(nix flake*)"
          "Bash(nix eval*)"
          "Bash(nix search*)"
          "Bash(nix repl*)"
          "Bash(darwin-rebuild build --flake .)"
          "Bash(darwin-rebuild check --flake .)"
          "Bash(git status)"
          "Bash(git diff*)"
          "Bash(git log*)"
          "Bash(ls*)"
          "Bash(find*)"
          "Bash(mkdir*)"
          "Bash(brew info*)"
          "Bash(brew list*)"
          "Bash(brew search*)"
        ];
        deny = [
          "Bash(rm -rf*)"
          "Bash(rm -fr*)"
          "Bash(rm -Rf*)"
          "Bash(rm -fR*)"
          "Bash(rm -r -f*)"
          "Bash(rm -f -r*)"
          "Bash(find * -delete*)"
          "Bash(find * -exec rm*)"
          "Bash(find * -execdir rm*)"
          "Bash(darwin-rebuild switch*)"
          "Bash(nix-collect-garbage*)"
          "Bash(nix store gc*)"
          "Bash(nix store delete*)"
          "Bash(nix-store --gc*)"
          "Bash(nix-store --delete*)"
          "Bash(git push*--force*)"
          "Bash(git push -f*)"
          "Bash(git push * -f*)"
          "Bash(git push * +*)"
          "Bash(git reset*)"
          "Bash(git clean*)"
          "Bash(git checkout -- *)"
          "Bash(git checkout .)"
          "Bash(gh auth token*)"
          "Bash(brew uninstall*)"
          "Bash(brew install*)"
        ];
      };
      promptSuggestionEnabled = false;
      skipAutoPermissionPrompt = true;
      skipDangerousModePermissionPrompt = true;
      skipWorkflowUsageWarning = true;
      spinnerVerbs = {
        mode = "replace";
        verbs = [
          "Fucking Shit Up"
          "One Big Cluster Fuck Coming Right Up"
          "Creating Some Quality AI Slop"
          "Taking A Big Shit On Your Codebase"
        ];
      };
      theme = "dark-ansi";
      tui = "fullscreen";
      useAutoModeDuringPlan = true;
      voice = {
        enabled = true;
        mode = "hold";
      };
      worktree.baseRef = "fresh";
    };
  };

  programs.pi-coding-agent = {
    enable = true;
    package = null;
    mutableSettings = true;
    settings = {
      collapseChangelog = true;
      enableInstallTelemetry = false;
      extensions = [ "-builtin:mcp" ];
      markdown.mermaid = "final";
      packages = [
        "git:github.com/mich-murphy/pi-extensions@main"
        "npm:@gotgenes/pi-subagents"
      ];
      quietStartup = true;
      retry.maxRetries = 6;
      showCacheMissNotices = true;
      theme = "dark";
      treeFilterMode = "no-tools";
    };
  };
}
