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
        enableAutosuggestions = true;
        enableSyntaxHighlighting = true;
        enableCompletion = true;
        defaultKeymap = "viins";
        history.size = 10000;
        initExtra = ''
          eval $(thefuck --alias)
          eval "$(direnv hook zsh)"
        '';
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
          gch = "g checkout";
          gst = "g stash";
          gl = "g log";
          gd = "g diff";
          rg = "batgrep";
          man = "batman";
          diff = "batdiff";
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
