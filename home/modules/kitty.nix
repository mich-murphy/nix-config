{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.kitty;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in {
  options.common.kitty = {
    enable = mkEnableOption "Enable kitty with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.kitty = {
      enable = true;
      package = fakepkg "kitty";
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
        macos_option_as_alt = "yes";
        allow_remote_control = "socket-only";
        listen_on = "unix:/tmp/kitty";
        shell_integration = "enabled";
      };
      extraConfig = ''
        # kitty-scrollback.nvim Kitten alias
        action_alias kitty_scrollback_nvim kitten /Users/mm/.local/share/nvim/lazy/kitty-scrollback.nvim/python/kitty_scrollback_nvim.py

        # browse scrollback buffer in nvim
        map kitty_mod+h kitty_scrollback_nvim
        # browse output of the last shell command in nvim
        map kitty_mod+g kitty_scrollback_nvim --config ksb_builtin_last_cmd_output
      '';
    };

    home.sessionVariables.TERMINAL = "kitty";
  };
}
