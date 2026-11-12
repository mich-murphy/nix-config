{
  config,
  lib,
  pkgs,
  ...
}:
let
  agentFile = config.lib.nixConfig.file "config/agents/AGENTS.md";
  skills = config.nixConfig.skills;
  skillNames = [
    "bro"
    "coordinator"
    "create-verification"
    "no-comments"
    "show-me"
  ];
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in
{
  home.file = {
    ".agents/skills".source = liveLink skills;
    ".claude/CLAUDE.md".source = agentFile;
    ".codex/AGENTS.md".source = agentFile;
    ".pi/agent/AGENTS.md".source = agentFile;
  }
  # Per-skill links keep ~/.claude/skills a real directory so Claude
  # Code's managed-skills sync cannot write into the skills repo.
  // lib.listToAttrs (
    map (name: {
      name = ".claude/skills/${name}";
      value.source = liveLink "${skills}/${name}";
    }) skillNames
  );

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
        DISABLE_ERROR_REPORTING = "1";
        DISABLE_TELEMETRY = "1";
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
          "Bash(sudo*)"
          "Read(~/.ssh/**)"
          "Read(**/.env)"
          "Read(**/.env.*)"
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
          "Priming The Slop Cannon"
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
      compaction.modelOverrides =
        lib.genAttrs
          (map (id: "claude-sdk/${id}") [
            "claude-5.5-sonnet"
            "claude-5.5-opus"
            "claude-5.2-fable"
            "claude-5.5-haiku"
          ])
          (_: {
            reserveTokens = 616000;
          });
      collapseChangelog = true;
      enableInstallTelemetry = false;
      extensions = [ ];
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

  home.activation.piSubagentsSettings = lib.hm.dag.entryAfter [ "linkGeneration" ] (
    lib.hm.generators.mkImpureConfigMerger {
      inherit pkgs;
      format = "json";
      empty = "{}";
      path = "${config.programs.pi-coding-agent.configDir}/subagents.json";
      staticSettings = (pkgs.formats.json { }).generate "pi-subagents.json" {
        defaultMaxTurns = 25;
        maxConcurrent = 3;
        unconsumedSessionRetentionMinutes = 120;
        wrapUpTurns = 5;
      };
      jqOperation = "$dynamic * $static";
    }
  );
}
