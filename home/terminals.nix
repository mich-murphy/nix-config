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

  # Ghostty wins over WezTerm's fallback, but a normal user definition can win.
  home.sessionVariables.TERMINAL =
    lib.mkOverride 900 "/Applications/Ghostty.app/Contents/MacOS/ghostty";
}
