{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.cli;
in
{
  options.common.cli = {
    enable = mkEnableOption "Enable personalised command line environment";
  };

  config = mkIf cfg.enable {
    programs = {
      zsh = {
        enable = true;
        dotDir = ".config/zsh";
        enableAutosuggestions = true;
        enableSyntaxHighlighting = true;
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
        envExtra = ''
          # fzf
          export FZF_DEFAULT_COMMAND='rg --files --hidden --glob "!.git"'
          export FZF_CTRL_T_COMMAND="$FZF_DEFAULT_COMMAND"

          FZF_COLORS="bg+:-1,\
          fg:gray,\
          fg+:white,\
          border:black,\
          spinner:0,\
          hl:cyan,\
          header:yellow,\
          info:blue,\
          pointer:blue,\
          marker:red,\
          prompt:gray,\
          hl+:blue"

          export FZF_DEFAULT_OPTS="--height 60% \
          --border sharp \
          --layout reverse \
          --color '$FZF_COLORS' \
          --prompt '∷ ' \
          --pointer ▶ \
          --marker ⇒"
          export FZF_ALT_C_OPTS="--preview 'tree -C {} | head -n 10'"
          export FZF_COMPLETION_DIR_COMMANDS="cd pushd rmdir tree ls"
        '';
        initExtra = ''
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
          }
          source ${pkgs.zsh-vi-mode}/share/zsh-vi-mode/zsh-vi-mode.plugin.zsh

          # the fuck config
          eval $(thefuck --alias)

          # direnv config
          eval "$(direnv hook zsh)"
        '';
        plugins = [
          {
            # https://github.com/hlissner/zsh-autopair
            name = "zsh-autopair";
            file = "zsh-autopair.plugin.zsh";
            src = pkgs.fetchFromGitHub {
              owner = "hlissner";
              repo = "zsh-autopair";
              rev = "396c38a7468458ba29011f2ad4112e4fd35f78e6";
              sha256 = "1h0vm2dgrmb8i2pvsgis3lshc5b0ad846836m62y8h3rdb3zmpy1";
            };
          }
        ];
        shellAliases = {
          ls = "lsd -lah";
          cat = "bat";
          vim = "nvim";
          g = "git";
          gs = "g status";
          ga = "g add";
          gc = "g commit";
          gp = "g push";
          gpl = "g pull";
          gb = "g branch";
          gco = "g checkout";
          gst = "g stash";
          gl = "g log";
          gd = "g diff";
          rg = "batgrep";
          man = "batman";
          diff = "batdiff";
          # detect yabai windows in space - make sure to add space no. after alias
          ybw = "yabai -m query --windows --space";
          drb = "darwin-rebuild switch --flake";
          dp = "nix run github:serokell/deploy-rs";
          agn = "nix run github:ryantm/agenix --";
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
          theme = "ansi";
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
      };
      lsd = {
        enable = true;
        settings = {
          classic = false;
          blocks = [
            "permission"
            "user"
            "size"
            "date"
            "name"
          ];
          date = "+%d %b %R";
          icons = {
            when = "auto";
            theme = "fancy";
            separator = " ";
          };
          layout = "grid";
          sorting = {
            column = "name";
            reverse = false;
            dir-grouping = "first";
          };
          symlink-arrow = "->";
        };
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
    };

    home.packages = with pkgs; [
      fd
      sd
      ripgrep
      jq
      tree
      thefuck
      du-dust
      grex
      delta
    ];
  };
}
