{
  lib,
  config,
  ...
}: let
  cfg = config.common.skhd;
in {
  options.common.skhd = {
    enable = lib.mkEnableOption "Enable SKHD hotkey daemon";
  };

  config = lib.mkIf cfg.enable {
    services = {
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
          # alt - return : /Applications/kitty.app/Contents/MacOS/kitty --single-instance -d ~
          alt - return : /Applications/WezTerm.app/Contents/MacOS/wezterm start --always-new-process
          shift + alt - return : '/Applications/Firefox Nightly.App/Contents/MacOS/firefox'

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
  };
}
