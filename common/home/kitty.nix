{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.kitty;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in
{
  options.common.kitty = {
    enable = mkEnableOption "Enable kitty with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.kitty = {
      enable = true;
      package = fakepkg "kitty";
      darwinLaunchOptions = [
        "--single-instance, -1"
        "--directory=/Users/mm"
        "--listen-on=unix:/tmp/my-socket"
      ];
      font = {
        name = "JetBrainsMono Nerd Font";
        size = 13;
      };
      theme = "Tokyo Night";
      settings = {
        disable_ligatures = "never";
        enable_audio_bell = "no";
        confirm_os_window_close = 0;
        window_padding_width = 10;
        scrollback_lines = 1000;
        hide_window_decorations = "titlebar-only";
        tab_bar_edge = "top";
      };
    };

    home.sessionVariables.TERMINAL = "kitty";
  };
}
