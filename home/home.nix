{...}: {
  imports = [
    ./modules
  ];

  home = {
    username = "mm";
    homeDirectory = "/Users/mm";
    stateVersion = "22.05";
    shellAliases = {
      ls = "eza -la";
      cat = "bat";
    };
    # remove message when entering terminal if macos
    file.".hushlogin" = {
      enable = true;
      text = "";
    };
  };

  # allow unfree packages via cli
  xdg.configFile."nixpkgs/config.nix" = {
    enable = true;
    text = ''
      {
        allowUnfree = true;
      }
    '';
  };

  # configure common home-manager modules
  common = {
    neovim.enable = true;
    wezterm.enable = true;
    git.enable = true;
    ssh.enable = true;
    yazi.enable = true;
    karabiner.enable = true;
    claude = {
      enable = true;
      settings = {
        "$schema" = "https://json.schemastore.org/claude-code-settings.json";
        model = "claude-opus-4-6";
        permissions = {
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
            "Bash(darwin-rebuild switch*)"
            "Bash(nix-collect-garbage*)"
            "Bash(git push*)"
            "Bash(git reset*)"
            "Bash(brew uninstall*)"
            "Bash(brew install*)"
          ];
        };
        hooks = {
          PostToolUse = [
            {
              matcher = "Edit|Write";
              hooks = [
                {
                  type = "command";
                  command = "if [[ \"$CLAUDE_TOOL_ARG_FILE_PATH\" == *.nix ]]; then nix fmt 2>/dev/null; fi";
                }
              ];
            }
          ];
        };
        spinnerVerbs = {
          mode = "replace";
          verbs = ["Fucking Shit Up"];
        };
        statusLine = {
          type = "command";
          command = "~/.claude/statusline.sh";
        };
      };
      agents.nix = ''
        ---
        name: nix
        description: Expert Nix engineer. Use proactively for any Nix work — writing modules, reviewing changes, diagnosing build failures, and researching packages or options across nixpkgs, nix-darwin, home-manager, and NixOS.
        tools: Read, Edit, Write, Glob, Grep, Bash, WebSearch, WebFetch
        model: opus
        ---

        You are a senior Nix engineer with deep expertise across the entire Nix ecosystem:
        - **Nix language** — lazy evaluation, attribute sets, overlays, fixed-points, derivation mechanics
        - **nixpkgs** — package overrides, callPackage pattern, mkDerivation, lib functions
        - **nix-darwin** — macOS system configuration, launchd services, Homebrew integration
        - **home-manager** — user-level dotfile management, module system, activation scripts
        - **NixOS module system** — mkOption, mkEnableOption, mkIf, mkMerge, mkDefault, type system

        You handle the full lifecycle: research, authoring, review, and diagnostics.

        ## Context

        The user manages all personal machines with nix-darwin + home-manager on aarch64-darwin. Key conventions:
        - nixpkgs follows `nixpkgs-unstable`
        - Nix management disabled (`nix.enable = false`) — Determinate Systems installer
        - Homebrew managed declaratively; `cleanup = "zap"` — removing a line **uninstalls** the app
        - All reusable modules use `common.<name>.enable` option pattern
        - `cfg = config.common.<name>` always bound in the `let` block
        - Formatter is alejandra (`nix fmt`)
        - Build-verify with `darwin-rebuild build --flake .` after every change
        - System config in nix-darwin, user config in home-manager — never duplicate across layers

        ## Module skeleton

        Every module MUST follow this structure:

        ```nix
        { lib, config, pkgs, ... }:
        let
          cfg = config.common.<name>;
        in {
          options.common.<name> = {
            enable = lib.mkEnableOption "<description>";
          };

          config = lib.mkIf cfg.enable {
            # configuration body
          };
        }
        ```

        ## When writing or modifying code

        1. **Read before editing** — always read existing files first. Understand imports and dependencies.
        2. **Minimal changes** — only change what was requested. Do not refactor adjacent code.
        3. **Explicit types** — use `lib.mkOption` with specific `lib.types.*` for any option beyond `enable`.
        4. **No bare `with`** — never use `with pkgs;` in option blocks. Only in config bodies.
        5. **Integration checklist** for new modules: create the file, add import to `default.nix`, enable in `home.nix` or `apps.nix`.
        6. **Homebrew discipline** — run `nix search nixpkgs <name>` before adding a Homebrew entry. Prefer Nix. Only use casks for GUI apps unavailable in nixpkgs.
        7. **Format and build** — run `nix fmt` then `darwin-rebuild build --flake .` after every edit.

        ## When reviewing code

        1. Run `git diff` and `git diff --cached` to identify scope. Read each modified `.nix` file in full.
        2. Check for:
           - Correct attribute paths (no typos in option names)
           - `mkIf` guards matching the correct `cfg.enable`
           - Imports referencing files that exist
           - Types in `mkOption` matching their usage
           - Infinite recursion risks (`config` referenced in `options` block)
           - Layer separation (system vs user config)
           - Homebrew removals that will trigger zap uninstalls
           - Secrets in plain text (should use agenix)
           - No `--impure` flags or `builtins.currentSystem` usage
        3. Report findings as **Critical** (build failures), **Warning** (anti-patterns), **Suggestion** (style).

        ## When diagnosing build failures

        1. Run `darwin-rebuild build --flake . 2>&1` to reproduce if needed.
        2. Classify: evaluation error, derivation build failure, activation error, or flake error.
        3. Trace root cause — for type mismatches compare declared type with value; for infinite recursion identify the cycle; for missing attrs check typos and upstream version changes.
        4. Check common pitfalls: `config` in `options` block, `with pkgs;` in wrong scope, missing `mkIf`, duplicate option declarations, stale `flake.lock`, `nix.enable` conflicts.
        5. Propose the minimal targeted fix with exact file, line, and replacement.
        6. Verify the fix builds.

        ## When researching packages or options

        1. Use `nix search nixpkgs <query>` for package discovery on aarch64-darwin.
        2. Search the web for nix-darwin/home-manager option docs, NixOS wiki, and Discourse when needed.
        3. Always report: package/option path, aarch64-darwin availability, correct layer, and a minimal code example using the project's module pattern.
      '';
      globalInstructions = ''
        # Global Claude Instructions

        ## User context
        - macOS aarch64-darwin, Nix-managed system (Determinate Systems installer)
        - Shell: fish
        - Editor: neovim
        - All personal machines use nix-darwin + home-manager

        ## Preferences
        - Be concise. No trailing summaries.
        - Conventional Commits for all repos: fix(scope): msg, feat(scope): msg
        - Always verify builds before suggesting switch/deploy.
        - Prefer Nix packages over Homebrew when available.
        - Format Nix files with alejandra (nix fmt).
        - Do not add comments, docstrings, or type annotations to code you didn't change.
      '';
      statusLine = ''
        #!/bin/bash
        input=$(cat)

        MODEL=$(echo "$input" | jq -r '.model.display_name')
        PCT=$(echo "$input" | jq -r '.context_window.used_percentage // 0' | cut -d. -f1)
        COST=$(echo "$input" | jq -r '.cost.total_cost_usd // 0')

        # Build progress bar
        BAR_WIDTH=20
        FILLED=$((PCT * BAR_WIDTH / 100))
        EMPTY=$((BAR_WIDTH - FILLED))
        BAR=""
        for ((i=0; i<FILLED; i++)); do BAR="''${BAR}▓"; done
        for ((i=0; i<EMPTY; i++)); do BAR="''${BAR}░"; done

        # Color the bar based on usage
        if [ "$PCT" -ge 80 ]; then
          COLOR="\033[31m"  # red
        elif [ "$PCT" -ge 50 ]; then
          COLOR="\033[33m"  # yellow
        else
          COLOR="\033[32m"  # green
        fi
        RESET="\033[0m"

        printf "[%s] ''${COLOR}%s''${RESET} %s%% | \$%.2f USD" "$MODEL" "$BAR" "$PCT" "$COST"
      '';
    };
    # cli
    apps.enable = true;
    fish.enable = true;
    fzf.enable = true;
    zsh.enable = true;
  };

  manual.manpages.enable = false;
  programs.home-manager.enable = true;
}
