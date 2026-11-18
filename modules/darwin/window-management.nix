{
  config,
  lib,
  pkgs,
  ...
}:
let
  user = config.system.primaryUser;

  chatApps = "^(Microsoft Teams|MSTeams|Slack)$";

  # Docked, Teams goes to the other display's first desktop and Slack to its
  # second. Undocked, both go to desktop 3. Desktops are looked up per
  # display, so the global numbers do not matter. With no argument it places
  # every Teams and Slack window; with a window id, only that one.
  placeChat = pkgs.writeShellApplication {
    name = "yabai-place-chat";
    runtimeInputs = [
      config.services.yabai.package
      pkgs.jq
    ];
    text = ''
      displays=$(yabai -m query --displays)
      if [[ $(jq length <<<"$displays") -gt 1 ]]; then
        spaces=$(jq -c '[.[] | select(.index == 2)][0].spaces' <<<"$displays")
        teams=$(jq '.[0]' <<<"$spaces")
        slack=$(jq '.[1] // .[0]' <<<"$spaces")
      else
        teams=$(jq '.[0].spaces | .[2] // .[-1]' <<<"$displays")
        slack=$teams
      fi

      if [[ $# -gt 0 ]]; then
        windows=$(yabai -m query --windows --window "$1" | jq -c '[.]')
      else
        windows=$(yabai -m query --windows)
      fi

      jq -r --arg apps ${lib.escapeShellArg chatApps} '.[]
        | select(.app | test($apps))
        | select(.subrole == "AXStandardWindow" and (."is-minimized" | not))
        | [.id, .app, .space, ."has-ax-reference"] | @tsv' <<<"$windows" |
        while IFS=$'\t' read -r id app space ax; do
          if [[ $app == Slack ]]; then target=$slack; else target=$teams; fi
          [[ $space == "$target" ]] && continue
          if [[ $ax != true ]]; then
            echo "yabai-place-chat: yabai cannot move $app window $id; restart yabai" >&2
            continue
          fi
          yabai -m window "$id" --space "$target" ||
            echo "yabai-place-chat: could not move $app window $id to desktop $target" >&2
        done
    '';
  };
in
{
  services.yabai = {
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
      # A rule only affects windows opened after it is added, and yabai
      # loads the open windows before it runs this file.
      yabai -m rule --apply

      # Teams and Slack: see placeChat. The display signals wait for macOS
      # to finish moving desktops and for the Dock restart below.
      yabai -m signal --add label=chat-window-created event=window_created app='${chatApps}' action='${lib.getExe placeChat} "$YABAI_WINDOW_ID"'
      yabai -m signal --add label=chat-display-added event=display_added action='sleep 5; ${lib.getExe placeChat}'
      yabai -m signal --add label=chat-display-removed event=display_removed action='sleep 5; ${lib.getExe placeChat}'
      ${lib.getExe placeChat} &

      # On macOS 27, Ctrl+3 and above stop switching desktops once a display
      # has connected or disconnected, until the Dock restarts. The delay
      # lets a burst of display changes finish first.
      yabai -m signal --add label=dock-restart-display-added event=display_added action="sleep 2; killall Dock"
      yabai -m signal --add label=dock-restart-display-removed event=display_removed action="sleep 2; killall Dock"
    '';
  };

  launchd.user.agents.yabai.serviceConfig = {
    StandardOutPath = "/tmp/yabai_${user}.out.log";
    StandardErrorPath = "/tmp/yabai_${user}.err.log";
  };
}
