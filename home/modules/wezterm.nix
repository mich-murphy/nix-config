{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.wezterm;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in {
  options.common.wezterm = {
    enable = lib.mkEnableOption "Enable Wezterm with personalised settings";
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.programs.kitty.enable == false;
        message = "Deactivate Kitty. Only one terminal emulator should be enabled";
      }
      {
        assertion = config.programs.alacritty.enable == false;
        message = "Deactivate Alacritty. Only one terminal emulator should be enabled";
      }
    ];
    programs.wezterm = {
      enable = true;
      enableZshIntegration = false;
      package = fakepkg "wezterm";
      extraConfig = ''
        local config = {}

        config.color_scheme = 'tokyonight_night'
        config.font = wezterm.font 'JetBrains Mono'
        config.font_size = 13.0
        config.hide_tab_bar_if_only_one_tab = true
        config.use_dead_keys = false
        config.window_decorations = "RESIZE"
        config.adjust_window_size_when_changing_font_size = false
        config.window_close_confirmation = 'NeverPrompt'

        return config;
      '';
    };

    home.sessionVariables.TERMINAL = "wezterm";
  };
}
