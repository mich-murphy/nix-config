{
  lib,
  config,
  ...
}: let
  cfg = config.common.herdr;
in {
  options.common.herdr = {
    enable = lib.mkEnableOption "Enable Herdr with personalised settings";
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."herdr/config.toml".text = ''
      [theme]
      name = "tokyo-night"

      [keys]
      prefix = "ctrl+a"

      split_vertical = "prefix+|"
      rename_tab = "prefix+comma"
      detach = "prefix+d"

      # Direct chords are process-aware plugin actions. Prefixed bindings
      # remain available as an escape hatch from applications that need them.
      focus_pane_left = "prefix+h"
      focus_pane_down = "prefix+j"
      focus_pane_up = "prefix+k"
      focus_pane_right = "prefix+l"

      [[keys.command]]
      key = "ctrl+h"
      type = "plugin_action"
      command = "herdr-splits.nav-left"
      description = "Navigate left (Neovim/Herdr)"

      [[keys.command]]
      key = "ctrl+j"
      type = "plugin_action"
      command = "herdr-splits.nav-down"
      description = "Navigate down (Neovim/Herdr)"

      [[keys.command]]
      key = "ctrl+k"
      type = "plugin_action"
      command = "herdr-splits.nav-up"
      description = "Navigate up (Neovim/Herdr)"

      [[keys.command]]
      key = "ctrl+l"
      type = "plugin_action"
      command = "herdr-splits.nav-right"
      description = "Navigate right (Neovim/Herdr)"

      [[keys.command]]
      # Keep Ctrl+L for navigation while Ctrl+Shift+L clears the focused pane.
      key = "ctrl+shift+l"
      type = "shell"
      command = "\"$HERDR_BIN_PATH\" pane send-keys \"$HERDR_ACTIVE_PANE_ID\" ctrl+l"
      description = "Clear focused pane"

      [ui]
      prompt_new_tab_name = false

      [ui.sound]
      enabled = false
    '';
  };
}
