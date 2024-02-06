{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.cli;
in {
  options.common.cli = {
    enable = mkEnableOption "Enable personalised command line environment";
  };

  config = mkIf cfg.enable {
    programs = {
      zsh = {
        enable = true;
        dotDir = ".config/zsh";
        enableAutosuggestions = true;
        syntaxHighlighting.enable = true;
        enableCompletion = true;
        historySubstringSearch.enable = true;
        defaultKeymap = "viins";
        history = {
          extended = true;
          share = true;
          expireDuplicatesFirst = true;
          ignoreDups = true;
          ignoreSpace = true;
          size = 10000;
        };
        autocd = true;
        oh-my-zsh = {
          enable = true;
          plugins = [
            "git"
            "tmux"
            "docker"
            "python"
            "pre-commit"
          ];
          extraConfig = ''
            source ${pkgs.zsh-autopair}/share/zsh/zsh-autopair/autopair.zsh
            autopair-init
          '';
        };
        envExtra = ''
          # fzf
          export FZF_COMPLETION_DIR_COMMANDS="cd pushd rmdir tree ls"
        '';
        initExtra = ''
          export PATH="/opt/homebrew/bin:$PATH"
          export LESS="--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT \
          --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4"

          # limit zcompdump to once daily
          autoload -Uz compinit
          for dump in ~/.zcompdump(N.mh+24); do
            compinit
          done
          compinit -C

          # navigation
          setopt AUTO_PUSHD
          setopt PUSHD_IGNORE_DUPS
          setopt PUSHD_SILENT
          setopt CORRECT
          setopt CDABLE_VARS

          # history (options unavailable in homemanager)
          setopt HIST_IGNORE_ALL_DUPS
          setopt HIST_FIND_NO_DUPS
          setopt HIST_SAVE_NO_DUPS
          setopt HIST_VERIFY

          # completions
          zstyle ':completion:*' completer _extensions _complete _approximate
          zstyle ':completion:*' use-cache on
          zstyle ':completion:*' cache-path "$XDG_CACHE_HOME/zsh/.zcompcache"
          zstyle ':completion:*' complete true
          zstyle ':completion:*' menu select
          zstyle ':completion:*' complete-options true
          zstyle ':completion:*' file-sort modification
          zstyle ':completion:*:*:*:*:corrections' format '%F{yellow}!- %d (errors: %e) -!%f'
          zstyle ':completion:*:*:*:*:descriptions' format '%F{blue}-- %D %d --%f'
          zstyle ':completion:*:*:*:*:messages' format ' %F{purple} -- %d --%f'
          zstyle ':completion:*:*:*:*:warnings' format ' %F{red}-- no matches found --%f'
          zstyle ':completion:*:*:cd:*' tag-order local-directories directory-stack path-directories
          zstyle ':completion:*' group-name '''
          zstyle ':completion:*:*:-command-:*:*' group-order aliases builtins functions commands
          zstyle ':completion:*' matcher-list ''' 'm:{a-zA-Z}={A-Za-z}' 'r:|[._-]=* r:|=*' 'l:|=* r:|=*'
          zstyle ':completion:*' keep-prefix true

          # zsh-vi-mode config
          function zvm_config() {
            ZVM_NORMAL_MODE_CURSOR=$ZVM_CURSOR_BLOCK
            ZVM_NORMAL_MODE_CURSOR=$ZVM_CURSOR_BLOCK
            ZVM_VI_SURROUND_BINDKEY=s-prefix
          }
          source ${pkgs.zsh-vi-mode}/share/zsh-vi-mode/zsh-vi-mode.plugin.zsh

          # show alias for future use
          source ${pkgs.zsh-you-should-use}/share/zsh/plugins/you-should-use/you-should-use.plugin.zsh

          # direnv config
          eval "$(direnv hook zsh)"
        '';
        shellAliases = {
          # general
          ls = "eza -la";
          cat = "bat";
          vim = "nvim";
          rg = "batgrep";
          man = "batman";
          diff = "batdiff";
          p = "less";
          # git
          gswr = "git switch-recent";
          geu = "git edit-unmerged";
          # kitty
          ssh = "kitty +kitten ssh";
          # detect yabai windows in space - make sure to add space no. after alias
          ybw = "yabai -m query --windows --space";
          # nix
          db = "darwin-rebuild switch --flake";
          dp = "nix run github:serokell/deploy-rs";
          agn = "nix run github:ryantm/agenix --";
          nph = "nix profile history --profile /nix/var/nix/profiles/system";
          ndh = "sudo nix profile wipe-history --profile /nix/var/nix/profiles/system --older-than 7d";
          ngc = "nix store gc";
        };
      };
      direnv = {
        enable = true;
        enableZshIntegration = true;
        nix-direnv.enable = true;
      };
      bat = {
        enable = true;
        config = {
          theme = "tokyonight";
        };
        themes = {
          tokyonight = {
            src =
              pkgs.fetchFromGitHub {
                owner = "folke";
                repo = "tokyonight.nvim";
                rev = "v2.9.0";
                sha256 = "NOKzXsY+DLNrykyy2Fr1eiSpYDiBIBNHL/7PPvbgbSo=";
              }
              + "/extras/sublime";
            file = "tokyonight_night.tmTheme";
          };
        };
        extraPackages = with pkgs.bat-extras; [
          batgrep
          batdiff
          batman
        ];
      };
      starship = {
        enable = true;
        enableZshIntegration = true;
        settings = {
          scan_timeout = 10;
        };
      };
      fzf = {
        enable = true;
        enableZshIntegration = true;
        tmux.enableShellIntegration = true;
        defaultCommand = ''rg --files --hidden --glob "!.git"'';
        fileWidgetCommand = "$FZF_DEFAULT_COMMAND";
        colors = {
          "bg+" = "#1a1b26";
          fg = "#a9b1d6";
          "fg+" = "#c0caf5";
          border = "#1a1b26";
          spinner = "#3b4261";
          hl = "#7dcfff";
          header = "#e0af68";
          info = "#7aa2f7";
          pointer = "#7aa2f7";
          marker = "#f7768e";
          prompt = "#a9b1d6";
          "hl+" = "#7aa2f7";
        };
        defaultOptions = [
          "--height 60%"
          "--border none"
          "--layout reverse"
          "--color '$FZF_COLORS'"
          "--prompt '∷ '"
          "--pointer ▶"
          "--marker ⇒"
        ];
        fileWidgetOptions = [
          "--height 60%"
          "--border none"
          "--no-scrollbar"
          "--inline-info"
          "--layout reverse"
          "--color '$FZF_COLORS'"
          "--prompt '∷ '"
          "--pointer ▶"
          "--marker ⇒"
          "--preview 'bat --color=always {}'"
          "--preview-window '~2',border-none"
        ];
        changeDirWidgetOptions = [
          "--preview 'tree -C {} | head -n 10'"
        ];
      };
      eza = {
        enable = true;
        icons = true;
        extraOptions = [
          "--group-directories-first"
        ];
      };
      zoxide = {
        enable = true;
        enableZshIntegration = true;
      };
      tealdeer = {
        enable = true;
        settings = {
          display = {
            compact = true;
          };
        };
      };
      btop = {
        enable = true;
        settings = {
          color_theme = "TTY";
          theme_background = false;
          vim_keys = true;
        };
      };
      zellij = {
        enable = true;
        settings = {
          theme = "tokyo-night";
          pane_frames = false;
        };
      };
      taskwarrior = {
        enable = true;
        colorTheme = "dark-gray-blue-256";
      };
    };

    home.packages = with pkgs; [
      fd
      sd
      ripgrep
      jq
      tree
      du-dust
      duf
      grex
      procs
      act
    ];
  };
}
