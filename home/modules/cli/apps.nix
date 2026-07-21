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
        enableFishIntegration = true;
        nix-direnv.enable = true;
      };
      bat = {
        enable = true;
        config = {
          # Use the terminal palette so bat follows Tokyo Night without a custom theme asset.
          theme = "ansi";
        };
      };
      starship = {
        enable = true;
        enableZshIntegration = true;
        enableFishIntegration = true;
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
        enableFishIntegration = true;
      };
      tealdeer = {
        enable = true;
        enableAutoUpdates = false;
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
