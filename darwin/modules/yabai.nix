{
  lib,
  config,
  ...
}: let
  cfg = config.common.yabai;
in {
  options.common.yabai = {
    enable = lib.mkEnableOption "Enable Yabai MacOS window manager";
  };

  config = lib.mkIf cfg.enable {
    services = {
      yabai = {
        enable = true;
        config = {
          # reference: https://github.com/koekeishiya/yabai/wiki/Configuration#configuration-file
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
          extraConfig = ''
            # rules
            # to identify windows use: yabai -m query --windows --space <int>
            yabai -m rule --add app='^Finder$' manage=off
            yabai -m rule --add app='^System Settings$' manage=off
            yabai -m rule --add app='^App Store$' manage=off
            yabai -m rule --add app='^Activity Monitor$' manage=off
            yabai -m rule --add app='^System Information$' manage=off
            yabai -m rule --add app='^Calculator$' manage=off
            yabai -m rule --add label="Dictionary" app="^Dictionary$" manage=off
            yabai -m rule --add label="Software Update" title="Software Update" manage=off
            yabai -m rule --add app="DisplayLinkUserAgent" title=".*" manage=off
            yabai -m rule --add title='^(Opening)' manage=off
            yabai -m rule --add app='Stats' manage=off
            yabai -m rule --add app='1Password' manage=off
            yabai -m rule --add app='Raycast' manage=off
            yabai -m config --space 5 layout float

            # enable scripting additions
            # yabai -m signal --add event=dock_did_restart action="sudo yabai --load-sa"
            # sudo yabai --load-sa

            # run applications on specified space
            # requires scripting additions active
            # yabai -m rule --add app='kitty' space=^1
            # yabai -m rule --add app='Firefox' space=^2
            # yabai -m rule --add app='Mail' space=3
            # yabai -m rule --add app='Messages' space=4
          '';
        };
      };
    };

    # install yabai scripting additions
    # environment.etc."sudoers.d/yabai".text = ''
    #   ${user} ALL = (root) NOPASSWD: ${pkgs.yabai}/bin/yabai --load-sa
    # '';
  };
}
