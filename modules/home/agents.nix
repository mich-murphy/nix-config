{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  # The agent CLIs come from numtide's llm-agents.nix on every host: it tracks
  # upstream's latest release daily and its cache has them prebuilt, where the
  # nixpkgs-unstable channel lags by days. A switch or deploy is their only
  # update: Claude Code's and Pi's wrappers turn their updaters off, and
  # Codex's check is off in its settings below.
  agentPkgs = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system};
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
  # Tokyo Night Night, as folke/tokyonight.nvim generates it for Pi, with some
  # changes. Pi finds a custom theme by `<name>.json`, so the name matches the
  # file. The rest are colours Pi draws as text or lines but upstream sets to
  # Neovim background colours, which hide them against the terminal or tool
  # background:
  # - `mdCodeBlock`, the text of unhighlighted code blocks, is `bg_dark`; ""
  #   uses the terminal's foreground instead.
  # - `toolDiff*`, the lines of edit diffs, are Neovim's diff backgrounds; they
  #   take the success, error and muted colours, as in Pi's built-in dark theme.
  # - `borderMuted`, Mermaid box borders and the editor border without
  #   thinking, is `border`, near-black; it takes the thinking-off border.
  # - `thinkingLow`, the editor border at low thinking, is `blue7`, darker than
  #   thinking off; it takes `border_highlight`, between minimal and medium.
  # - `userMessageBg` is `bg_dark`, barely darker than the terminal, so sent
  #   messages hardly stand out; it takes `selectedBg`, as Pi's built-in dark
  #   theme gives user messages and selection the same blue.
  piThemeName = "tokyonight-night";
  upstreamPiTheme = lib.importJSON "${inputs.tokyonight}/extras/pi/tokyonight_night.json";
  piTheme = lib.recursiveUpdate upstreamPiTheme {
    name = piThemeName;
    colors = with upstreamPiTheme.colors; {
      mdCodeBlock = "";
      toolDiffAdded = success;
      toolDiffRemoved = error;
      toolDiffContext = muted;
      borderMuted = thinkingOff;
      thinkingLow = border;
      userMessageBg = selectedBg;
    };
  };
in
{
  home.file = {
    ".agents/skills".source = liveLink skills;
    ".claude/CLAUDE.md".source = agentFile;
    ".codex/AGENTS.md".source = agentFile;
    ".pi/agent/AGENTS.md".source = agentFile;
    "${config.programs.pi-coding-agent.configDir}/themes/${piThemeName}.json".source =
      (pkgs.formats.json { }).generate "${piThemeName}.json"
        piTheme;
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
    package = agentPkgs.claude-code;
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

  programs.codex = {
    enable = true;
    package = agentPkgs.codex;
    # config.toml also holds per-project trust and hook state that Codex
    # writes, so only this key is merged in. It stops the startup check that
    # announces a new release.
    mutableSettings = true;
    settings.check_for_update_on_startup = false;
  };

  programs.pi-coding-agent = {
    enable = true;
    package = agentPkgs.pi;
    # Pi installs its npm and git packages with npm.
    extraPackages = [ pkgs.nodejs ];
    mutableSettings = true;
    settings = {
      compaction.modelOverrides =
        lib.genAttrs
          (map (id: "claude/${id}") [
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
      theme = piThemeName;
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
