{ config, pkgs, ... }:
let
  palette = config.lib.tokyonight.palette;
in
{
  home.packages = [
    pkgs.curl
    pkgs.dust
    pkgs.fd
    pkgs.just
    pkgs.jq
    pkgs.ouch
    pkgs.procs
    pkgs.prek
    pkgs.ripgrep
    pkgs.rsync
    pkgs.sd
    pkgs.tree
    pkgs.wget
  ];

  programs = {
    direnv = {
      enable = true;
      nix-direnv.enable = true;
    };
    bat = {
      enable = true;
      config = {
        theme = "tokyonight_night";
      };
      themes.tokyonight_night = {
        src = config.lib.tokyonight.extras;
        file = "sublime/tokyonight_night.tmTheme";
      };
    };
    starship = {
      enable = true;
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
    zoxide.enable = true;
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
    fzf = {
      enable = true;
      tmux.enableShellIntegration = true;
      fileWidget.command = ''rg --files --hidden --glob "!.git"'';
      colors = {
        "bg+" = palette.bg;
        fg = palette.fg_dark;
        "fg+" = palette.fg;
        border = palette.bg;
        spinner = palette.fg_gutter;
        hl = palette.cyan;
        header = palette.yellow;
        info = palette.blue;
        pointer = palette.blue;
        marker = palette.red;
        prompt = palette.fg_dark;
        "hl+" = palette.blue;
      };
      defaultOptions = [
        "--height 60%"
        "--border none"
        "--layout reverse"
      ];
      fileWidget.options = [
        "--no-scrollbar"
        "--inline-info"
        "--preview 'bat --color=always {}'"
        "--preview-window '~2',border-none"
      ];
      changeDirWidget.options = [
        "--preview 'tree -C {} | head -n 10'"
      ];
    };
  };
}
