{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.yabai;
  # nix-prefetch-url --unpack $YOUR_URL
  yabai = pkgs.yabai.overrideAttrs (old: {
    version = "5.0.6";
    src = builtins.fetchTarball {
      url ="https://github.com/koekeishiya/yabai/releases/download/v5.0.6/yabai-v5.0.6.tar.gz";
      sha256 = "1szyjcwkhn2wbrcfhh9lh5bnfm1cavxrx6xj4q7521z3zj29a9kf";
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
          extraConfig = ''
            # RULES
            # Some of these guys are hidden and supper irritating to find. 
            # Use `yabai -m query --windows --space <int>`
            yabai -m rule --add app='^Finder$' manage=off layer=above
            yabai -m rule --add app='^System Settings$' manage=off layer=above
            yabai -m rule --add app='^App Store$' manage=off layer=above
            yabai -m rule --add app='^Activity Monitor$' manage=off layer=above
            yabai -m rule --add app='^System Information$' manage=off layer=above
            yabai -m rule --add app='^Calculator$' manage=off layer=above
            yabai -m rule --add label="Dictionary" app="^Dictionary$" manage=off
            yabai -m rule --add label="Software Update" title="Software Update" manage=off
            yabai -m rule --add app="DisplayLinkUserAgent" title=".*" manage=off
            yabai -m rule --add title='^(Opening)' manage=off layer=above
            yabai -m rule --add app='Alfred' manage=off layer=above
            yabai -m rule --add app='Stats' manage=off layer=above
          '';
        };
      };
      skhd = {
        enable = true;
        skhdConfig = ''
          # Applications Shortcuts
          cmd - return : /Applications/Alacritty.App/Contents/MacOS/Alacritty
          cmd + shift - return : /Applications/Firefox.App/Contents/MacOS/firefox
          # Toggle Window
          lalt - t : yabai -m window --toggle float; yabai -m window --grid 4:4:1:1:2:2
          lalt - f : yabai -m window --toggle zoom-fullscreen
          lalt + shift - f : yabai -m window --toggle native-fullscreen
          lalt - q : yabai -m window --close
          # Toggle Gaps
          lalt - g : yabai -m space --toggle padding; yabai -m space --toggle gap
          # Focus Window
          lalt - k : yabai -m window --focus north
          lalt - j : yabai -m window --focus south
          lalt - h : yabai -m window --focus west
          lalt - l : yabai -m window --focus east
          # Swap Window
          shift + lalt - k : yabai -m window --swap north
          shift + lalt - j : yabai -m window --swap south
          shift + lalt - h : yabai -m window --swap west
          shift + lalt - l : yabai -m window --swap east
          # Resize Window
          lalt + cmd - h : yabai -m window --resize left:-50:0; yabai -m window --resize right:-50:0
          lalt + cmd - l : yabai -m window --resize left:50:0; yabai -m window --resize right:50:0
          lalt + cmd - k : yabai -m window --resize bottom:0:-50; yabai -m window --resize top:0:-50
          lalt + cmd - j : yabai -m window --resize bottom:0:50; yabai -m window --resize top:0:50
          # Balance All Windows
          lalt + cmd - e : yabai -m space --balance
          # Send to Space
          shift + lctrl - 1 : yabai -m window --space 1
          shift + lctrl - 2 : yabai -m window --space 2
          shift + lctrl - 3 : yabai -m window --space 3
          shift + lctrl - 4 : yabai -m window --space 4
          shift + lctrl - 5 : yabai -m window --space 5
        '';
      };
    };
  };
}
