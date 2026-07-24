{
  lib,
  pkgs,
  repoRoot,
  ...
}: {
  services = {
    yabai = {
      enable = true;
      package = pkgs.yabai.overrideAttrs (_old: {
        enableParallelBuilding = false;
      });
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
        window_shadow = "float";
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
      '';
    };

    # Keep nix-darwin's package and LaunchAgent, but point it at the live file.
    skhd = {
      enable = true;
      skhdConfig = "";
    };
  };

  launchd.user.agents.skhd.serviceConfig.ProgramArguments = lib.mkForce [
    "${pkgs.skhd}/bin/skhd"
    "-c"
    "${repoRoot}/config/skhd/skhdrc"
  ];
}
