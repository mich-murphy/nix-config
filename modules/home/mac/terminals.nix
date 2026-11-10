{
  config,
  lib,
  ...
}:
{
  xdg.configFile = {
    "ghostty".source = config.lib.nixConfig.file "config/ghostty";
  };

  # Ghostty's macOS app prefers this native path over the XDG path. Manage
  # both so the GUI and CLI resolve the same live configuration.
  home.file."Library/Application Support/com.mitchellh.ghostty/config.ghostty".source =
    config.lib.nixConfig.file "config/ghostty/config";

  home.sessionVariables = {
    # Avoid Nix profile paths that Codex cannot inspect from its restricted
    # filesystem. Ghostty continues to provide its bundled terminfo via TERMINFO.
    TERMINFO_DIRS = "/usr/share/terminfo";

    # Prefer Ghostty while allowing a regular-priority definition to win.
    TERMINAL = lib.mkOverride 900 "/Applications/Ghostty.app/Contents/MacOS/ghostty";
  };
}
