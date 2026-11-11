{
  config,
  lib,
  pkgs,
  ...
}:
let
  checkout = config.home-manager.users.${config.system.primaryUser}.nixConfig.checkout;
in
{
  services = {
    yabai = {
      enable = true;
      config = {
        focus_follows_mouse = "off";
        mouse_follows_focus = "off";
        mouse_modifier = "fn";
        mouse_action1 = "move";
        mouse_action2 = "resize";
        layout = "bsp";
        split_ratio = 0.5;
        auto_balance = "off";
        top_padding = 5;
        bottom_padding = 5;
        left_padding = 5;
        right_padding = 5;
        window_gap = 5;
        window_placement = "second_child";
      };
      extraConfig = ''
        yabai -m rule --add app='^Finder$' manage=off
        yabai -m rule --add app='^System Settings$' manage=off
        yabai -m rule --add app='^App Store$' manage=off
        yabai -m rule --add app='^Activity Monitor$' manage=off
        yabai -m rule --add app='^System Information$' manage=off
        yabai -m rule --add app='^Calculator$' manage=off
        yabai -m rule --add label="Dictionary" app="^Dictionary$" manage=off
        yabai -m rule --add label="Software Update" title="Software Update" manage=off
        yabai -m rule --add title='^(Opening)' manage=off
        yabai -m rule --add app='Stats' manage=off
        yabai -m rule --add app='1Password' manage=off
        yabai -m rule --add app='^Archive Utility$' manage=off
        yabai -m rule --add app='^Preview$' manage=off
        yabai -m rule --add app='^UTM$' manage=off
        yabai -m rule --add app='^YubiKey Manager$' manage=off
        yabai -m config --space 5 layout float

        # On macOS 27, Ctrl+3 and above stop switching desktops once a display
        # has connected or disconnected, until the Dock restarts. The delay
        # lets a burst of display changes finish first.
        yabai -m signal --add label=dock-restart-display-added event=display_added action="sleep 2; killall Dock"
        yabai -m signal --add label=dock-restart-display-removed event=display_removed action="sleep 2; killall Dock"
      '';
    };

    skhd.enable = true;
  };

  launchd.user.agents.skhd.serviceConfig.ProgramArguments = lib.mkForce [
    "${pkgs.skhd}/bin/skhd"
    "-c"
    "${checkout}/config/skhd/skhdrc"
  ];
}
