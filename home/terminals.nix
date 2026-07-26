{
  config,
  lib,
  repoRoot,
  ...
}: {
  assertions = [
    {
      assertion = !config.programs.kitty.enable;
      message = "Deactivate Kitty. Only one terminal emulator should be enabled";
    }
    {
      assertion = !config.programs.alacritty.enable;
      message = "Deactivate Alacritty. Only one terminal emulator should be enabled";
    }
  ];

  xdg.configFile = {
    "ghostty".source =
      config.lib.file.mkOutOfStoreSymlink "${repoRoot}/config/ghostty";
    "wezterm".source =
      config.lib.file.mkOutOfStoreSymlink "${repoRoot}/config/wezterm";
  };

  home.sessionVariables = {
    # Avoid Nix profile paths that Codex cannot inspect from its restricted
    # filesystem. Ghostty continues to provide its bundled terminfo via TERMINFO.
    TERMINFO_DIRS = "/usr/share/terminfo";

    # Prefer Ghostty while allowing a regular-priority definition to win.
    TERMINAL =
      lib.mkOverride 900 "/Applications/Ghostty.app/Contents/MacOS/ghostty";
  };
}
