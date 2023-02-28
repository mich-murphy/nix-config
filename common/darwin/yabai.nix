{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.yabai;
  yabai = pkgs.yabai.overrideAttrs (old: rec {
    version = "5.0.2";
    src = builtins.fetchTarball {
      url = https://github.com/koekeishiya/yabai/releases/download/v5.0.2/yabai-v5.0.2.tar.gz;
      sha256 = "13q8awbmp2vb1f9iycbvlfd5c2fmk5786cwm40bv6zwi4w8bplgy";
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
            yabai -m rule --add title='Preferences' manage=off layer=above
            yabai -m rule --add title='^(Opening)' manage=off layer=above
            yabai -m rule --add title='Library' manage=off layer=above
            yabai -m rule --add app='^Calculator$' manage=off layer=above
            yabai -m rule --add app='^App Store$' manage=off layer=above
            yabai -m rule --add app='^System Preferences$' manage=off layer=above
            yabai -m rule --add app='^Activity Monitor$' manage=off layer=above
            yabai -m rule --add app='Finder' manage=off layer=above
            yabai -m rule --add app='Alfred' manage=off layer=above
            yabai -m rule --add app='1Password' manage=off layer=above
            yabai -m rule --add app='^System Information$' manage=off layer=above
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
