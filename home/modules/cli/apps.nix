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
          theme = "carbonfox";
        };
        themes = {
          carbonfox = {
            src =
              pkgs.fetchFromGitHub {
                owner = "EdenEast";
                repo = "nightfox.nvim";
                rev = "ba47d4b4c5ec308718641ba7402c143836f35aa9";
                sha256 = "sha256-HoZEwncrUnypWxyB+XR0UQDv+tNu1/NbvimxYGb7qu8=";
              }
              + "/extra/carbonfox";
            file = "carbonfox.tmTheme";
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
