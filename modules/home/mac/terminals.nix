{ config, ... }:
{
  xdg.configFile = {
    "ghostty".source = config.lib.nixConfig.file "config/ghostty";
  };

  home.file."Library/Application Support/com.mitchellh.ghostty/config.ghostty".source =
    config.lib.nixConfig.file "config/ghostty/config";

  home.sessionVariables = {
    TERMINFO_DIRS = "/usr/share/terminfo";

    TERMINAL = "/Applications/Ghostty.app/Contents/MacOS/ghostty";
  };
}
