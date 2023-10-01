{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.yabai;
  # nix-prefetch-url --unpack $YOUR_URL
  yabai = pkgs.yabai.overrideAttrs (final: {
    version = "5.0.8";
    src = builtins.fetchTarball {
      url ="https://github.com/koekeishiya/yabai/releases/download/v5.0.8/yabai-v5.0.8.tar.gz";
      sha256 = "11dimky2r0gskp8vniwjc4d70bpkkck5mnjwcbjxid2csykhrx8p";
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
          # defines a new mode 'test' with an on_enter command, that captures keypresses
          :: resize @
          # from 'default' mode, activate mode 'resize'
          cmd - r ; resize
          # from 'resize' mode, activate mode 'default'
          resize < cmd - r ; default
          resize < h : yabai -m window --resize left:-50:0; yabai -m window --resize right:-50:0
          resize < l : yabai -m window --resize left:50:0; yabai -m window --resize right:50:0
          resize < k : yabai -m window --resize bottom:0:-50; yabai -m window --resize top:0:-50
          resize < j : yabai -m window --resize bottom:0:50; yabai -m window --resize top:0:50
          resize < e : yabai -m space --balance

          # Applications Shortcuts
          cmd - return : /Applications/kitty.app/Contents/MacOS/kitty --single-instance -d ~
          cmd + shift - return : /Applications/Firefox.App/Contents/MacOS/firefox
          # Toggle Window
          cmd - t : yabai -m window --toggle float; yabai -m window --grid 4:4:1:1:2:2
          cmd - f : yabai -m window --toggle zoom-fullscreen
          alt - q : yabai -m window --close
          # Toggle Gaps
          cmd - g : yabai -m space --toggle padding; yabai -m space --toggle gap
          # Balance All Windows
          cmd - e : yabai -m space --balance

          # Focus Window
          cmd - k : yabai -m window --focus north || yabai -m display --focus north
          cmd - j : yabai -m window --focus south || yabai -m display --focus south
          cmd - h : yabai -m window --focus west || yabai -m display --focus west
          cmd - l : yabai -m window --focus east || yabai -m display --focus east
          # Swap Window
          cmd + shift - k : yabai -m window --swap north || yabai -m window --display north
          cmd + shift - j : yabai -m window --swap south || yabai -m window --display south
          cmd + shift - h : yabai -m window --swap west || yabai -m window --display west
          cmd + shift - l : yabai -m window --swap east || yabai -m window --display east
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
