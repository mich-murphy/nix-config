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
            "Bash(darwin-rebuild build --flake .)"
            "Bash(git status)"
            "Bash(git diff*)"
            "Bash(git log*)"
            "Bash(nix search*)"
          ];
          deny = [
            "Bash(rm -rf*)"
            "Bash(darwin-rebuild switch*)"
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
        [ "$FILLED" -gt 0 ] && BAR=$(printf "%''${FILLED}s" | tr ' ' '▓')
        [ "$EMPTY" -gt 0 ] && BAR="''${BAR}$(printf "%''${EMPTY}s" | tr ' ' '░')"

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
