{ lib, config, pkgs, user, ... }:

with lib;

let
  cfg = config.common.yabai;
  # nix-prefetch-url --unpack $YOUR_URL
  yabai = pkgs.yabai.overrideAttrs (final: {
    version = "5.0.9";
    src = builtins.fetchTarball {
      url ="https://github.com/koekeishiya/yabai/releases/download/v5.0.9/yabai-v5.0.9.tar.gz";
      sha256 = "17rb1jygs1g0pwl2s3nyslbwnpw809is5ilqxz899agl97gwka37";
    };
  });
in
{
  options.common.yabai = {
    enable = mkEnableOption "Enable Yabai MacOS window manager and SKHD hotkey daemon";
  };

  config = mkIf cfg.enable {
    services = {
      yabai = {
        enable = true; 
        package = yabai;
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
          # window_border = "on";
          # window_border_width = 2;
          # window_border_radius = 11;
          # window_border_blur = "off";
          # active_window_border_color = "0xff9d7cd8";
          # normal_window_border_color = "0xff414868";
          extraConfig = ''
            # RULES
            # Some of these guys are hidden and supper irritating to find. 
            # Use `yabai -m query --windows --space <int>`
            # yabai -m signal --add event=dock_did_restart action="sudo yabai --load-sa"
            # sudo yabai --load-sa
            yabai -m rule --add app='^Finder$' manage=off border=off
            yabai -m rule --add app='^System Settings$' manage=off border=off
            yabai -m rule --add app='^App Store$' manage=off border=off
            yabai -m rule --add app='^Activity Monitor$' manage=off border=off
            yabai -m rule --add app='^System Information$' manage=off border=off
            yabai -m rule --add app='^Calculator$' manage=off border=off
            yabai -m rule --add label="Dictionary" app="^Dictionary$" manage=off border=off
            yabai -m rule --add label="Software Update" title="Software Update" manage=off border=off
            yabai -m rule --add app="DisplayLinkUserAgent" title=".*" manage=off border=off
            yabai -m rule --add title='^(Opening)' manage=off border=off
            yabai -m rule --add app='Alfred' manage=off border=off
            yabai -m rule --add app='Stats' manage=off border=off
            yabai -m rule --add app='1Password' manage=off border=off
            yabai -m rule --add app='Raycast' manage=off border=off
          '';
        };
      };
      skhd = {
        enable = true;
        skhdConfig = ''
          # defines a new mode 'resize' with an on_enter command, that captures keypresses
          :: resize @
          alt - r ; resize
          resize < alt - r ; default
          resize < h : yabai -m window --resize left:-50:0; yabai -m window --resize right:-50:0
          resize < l : yabai -m window --resize left:50:0; yabai -m window --resize right:50:0
          resize < k : yabai -m window --resize bottom:0:-50; yabai -m window --resize top:0:-50
          resize < j : yabai -m window --resize bottom:0:50; yabai -m window --resize top:0:50
          resize < o : yabai -m space --rotate 90
          resize < e : yabai -m space --balance

          # Applications Shortcuts
          alt - return : /Applications/kitty.app/Contents/MacOS/kitty --single-instance -d ~
          shift + alt - return : /Applications/Firefox.App/Contents/MacOS/firefox
          # Toggle Window
          alt - t : yabai -m window --toggle float; yabai -m window --grid 4:4:1:1:2:2
          alt - f : yabai -m window --toggle zoom-fullscreen
          alt - q : yabai -m window --close
          alt - g : yabai -m space --toggle padding; yabai -m space --toggle gap

          # Focus Window
          alt - k : yabai -m window --focus north || yabai -m display --focus north
          alt - j : yabai -m window --focus south || yabai -m display --focus south
          alt - h : yabai -m window --focus west || yabai -m display --focus west
          alt - l : yabai -m window --focus east || yabai -m display --focus east

          # Swap Window
          shift + alt - k : yabai -m window --swap north || yabai -m window --display north
          shift + alt - j : yabai -m window --swap south || yabai -m window --display south
          shift + alt - h : yabai -m window --swap west || yabai -m window --display west
          shift + alt - l : yabai -m window --swap east || yabai -m window --display east

          # Send to Space
          shift + ctrl - 1 : yabai -m window --space 1 --focus
          shift + ctrl - 2 : yabai -m window --space 2 --focus
          shift + ctrl - 3 : yabai -m window --space 3 --focus
          shift + ctrl - 4 : yabai -m window --space 4 --focus
          shift + ctrl - 5 : yabai -m window --space 5 --focus
        '';
      };
    };

    # environment.etc."sudoers.d/yabai".text = ''
    #   ${user} ALL = (root) NOPASSWD: ${pkgs.yabai}/bin/yabai --load-sa
    # '';
  };
}
