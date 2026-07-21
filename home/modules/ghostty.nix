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
      window-decoration = false
      macos-titlebar-style = transparent
      window-theme = dark
    '';

    home.sessionVariables.TERMINAL = "/Applications/Ghostty.app/Contents/MacOS/ghostty";
  };
}
