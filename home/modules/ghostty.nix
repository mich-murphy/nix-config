{
  lib,
  config,
  ...
}: let
  cfg = config.common.ghostty;
in {
  options.common.ghostty = {
    enable = lib.mkEnableOption "Enable Ghostty with personalised settings";
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."ghostty/config".text = ''
      font-family = Berkeley Mono
      font-size = 13
      theme = TokyoNight Night
      window-decoration = auto
      macos-titlebar-style = hidden
      macos-window-shadow = false
      window-theme = dark
      window-padding-x = 10
      confirm-close-surface = false

      # Preserve Ctrl+Shift+L as a distinct key inside Herdr and shells.
      keybind = ctrl+shift+key_l=csi:24~
    '';

    home.sessionVariables.TERMINAL = "/Applications/Ghostty.app/Contents/MacOS/ghostty";
  };
}
