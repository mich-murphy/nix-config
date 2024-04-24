{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.zsh;
in {
  options.common.zsh = {
    enable = mkEnableOption "Enable personalised zshrc";
  };

  config = mkIf cfg.enable {
    programs.zsh = {
      enable = true;
      dotDir = ".config/zsh";
      autosuggestion.enable = true;
      syntaxHighlighting.enable = true;
      enableCompletion = true;
      historySubstringSearch.enable = true;
      defaultKeymap = "viins"; # use vim keys
      history = {
        extended = true;
        share = true;
        expireDuplicatesFirst = true;
        ignoreDups = true;
        ignoreSpace = true;
        size = 10000;
      };
      autocd = true;
      # use oh-my-zsh for aliases
      oh-my-zsh = {
        enable = true;
        plugins = [
          "git"
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
        zl = "zellij";
      };
    };
  };
}
