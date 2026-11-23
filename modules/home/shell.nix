{ config, pkgs, ... }:
{
  programs.fish = {
    enable = true;

    shellInit = ''
      fish_add_path --append --path $HOME/.local/bin
    '';

    interactiveShellInit = ''
      set -g fish_key_bindings fish_vi_key_bindings
      set -g fish_greeting

      bind --mode insert f12 clear-screen
      bind --mode default f12 clear-screen

      set -g fish_cursor_default block
      set -g fish_cursor_insert line
      set -g fish_cursor_replace_one underscore
      set -g fish_cursor_visual block

      ${builtins.readFile "${config.lib.tokyonight.extras}/fish/tokyonight_night.fish"}
    '';

    plugins = [
      {
        name = "autopair";
        src = pkgs.fishPlugins.autopair-fish.src;
      }
      {
        name = "fzf-fish";
        src = pkgs.fishPlugins.fzf-fish.src;
      }
    ];
  };
}
