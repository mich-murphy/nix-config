{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.apps;
in {
  options.common.apps = {
    enable = lib.mkEnableOption "Enable personalised CLI utils";
  };

  config = lib.mkIf cfg.enable {
    home.packages = with pkgs; [
      fd
      sd
      ripgrep
      jq
      tree
      procs
      ouch
    ];

    programs = {
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
                rev = "v3.0.1";
                sha256 = "sha256-QKqCsPxUyTur/zOUZdiT1cOMSotmTsnOl/3Sn2/NlUI=";
              }
              + "/extras/sublime";
            file = "tokyonight_night.tmTheme";
          };
        };
      };
      starship = {
        enable = true;
        enableZshIntegration = true;
        settings = {
          scan_timeout = 10;
          git_status = {
            deleted = "";
          };
        };
      };
      eza = {
        enable = true;
        icons = "auto";
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
    };
  };
}
