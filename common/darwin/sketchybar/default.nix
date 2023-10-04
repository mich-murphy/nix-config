{ lib, config, ... }:

with lib;

let
  cfg = config.common.sketchybar;
  scripts = ./scripts;
in
{
  options.common.sketchybar = {
    enable = mkEnableOption "Enable Sketchybar bar replacement for MacOS";
  };

  config = mkIf cfg.enable {
    services.sketchybar = {
      enable = true;
      config = ''
        #!/bin/bash

        scripts="${scripts}"

        bar_color=0xff1a1b26
        # bar_color=0x30000000
        icon_font="JetBrainsMono Nerd Font:Medium:13.0"
        icon_color=0xffc0caf5
        icon_highlight_color=0xffebcb8b
        label_font="$icon_font"
        label_color="$icon_color"
        label_highlight_color="$icon_highlight_color"

        spaces=()
        for i in {1..8}
        do
            spaces+=(--add space space$i left \
              --set space$i \
                associated_display=1 \
                associated_space=$i \
                icon=$i \
                click_script="yabai -m space --focus $i" \
                script="$scripts/space.sh")
        done

        sketchybar -m \
          --bar \
            height=35 \
            position=top \
            sticky=on \
            shadow=on \
            notch_width=200 \
            padding_left=20 \
            padding_right=20 \
            color="$bar_color" \
          --default \
            icon.font="$icon_font" \
            icon.color="$icon_color" \
            icon.highlight_color="$icon_highlight_color" \
            label.font="$label_font" \
            label.color="$label_color" \
            label.highlight_color="$label_highlight_color" \
            icon.padding_left=10 \
            icon.padding_right=6 \
          --add item clock right \
          --set clock update_freq=10 script="$scripts/status.sh" icon.padding_left=2 \
          --add item battery right \
          --set battery update_freq=120 script="$scripts/battery.sh" \
          --subscribe battery system_woke power_source_change \
          --add item wifi right \
          --set wifi script="$scripts/wifi.sh" click_script="$scripts/click-wifi.sh" \
          --subscribe wifi wifi_change \
          --add item network right \
          --default \
            icon.padding_left=0 \
            icon.padding_right=2 \
            label.padding_right=16 \
          "''${spaces[@]}" \
          --add item separator left \
          --set separator icon= \
          --add item title left \
          --set title script='sketchybar --set "$NAME" label="$INFO"' \
          --subscribe title front_app_switched \

        sketchybar --update

        # ram disk
        cache="$HOME/.cache/sketchybar"
        mkdir -p "$cache"
        if ! mount | grep -qF "$cache"
        then
          disk=$(hdiutil attach -nobrowse -nomount ram://1024)
          disk="''${disk%% *}"
          newfs_hfs -v sketchybar "$disk"
          mount -t hfs -o nobrowse "$disk" "$cache"
        fi
      '';
    };

    services.yabai.config.external_bar = "all:35:0";
    system.defaults.NSGlobalDomain._HIHideMenuBar = true;
  };
}
