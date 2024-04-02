{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.zellij;
in {
  options.common.zellij = {
    enable = mkEnableOption "Enable Zellij with personalised configuration";
  };

  config = mkIf cfg.enable {
    programs.zellij.enable = true;

    # add zellij configuration to .config/
    xdg.configFile = mkIf config.programs.zellij.enable {
      "zellij/config.kdl" = {
        enable = true;
        target = "zellij/config.kdl";
        text = ''
          keybinds {
            unbind "Ctrl t"
            tab {
              bind "Ctrl w" { SwitchToMode "Normal"; }
            }
            shared_except {
              bind "Alt h" { MoveFocusOrTab "Left"; }
              bind "Alt j" { MoveFocusOrTab "Down"; }
              bind "Alt k" { MoveFocusOrTab "Up"; }
              bind "Alt l" { MoveFocusOrTab "Right"; }
              bind "Alt w" { ToggleFloatingPanes; SwitchToMode "Normal"; }
            }
            shared_except "tab" "locked" {
              bind "Ctrl w" { SwitchToMode "Tab"; }
            }
          }
          pane_frames false
          theme "tokyo-night-dark"
        '';
      };
    };
  };
}
