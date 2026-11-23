{
  config,
  lib,
  pkgs,
  ...
}:
let
  user = config.system.primaryUser;

  chatApps = "^(Microsoft Teams|MSTeams|Slack)$";
  dockRestartDelay = 2;
  chatPlacementDelay = toString (dockRestartDelay + 3);

  placeChat = pkgs.writeShellApplication {
    name = "yabai-place-chat";
    runtimeInputs = [
      config.services.yabai.package
      pkgs.jq
    ];
    text = ''
      displays=$(yabai -m query --displays)
      if [[ $(jq length <<<"$displays") -gt 1 ]]; then
        target=$(jq '[.[] | select(.index == 2)][0].spaces[0]' <<<"$displays")
      else
        target=$(jq '.[0].spaces | .[2] // .[-1]' <<<"$displays")
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
      yabai -m rule --apply

      yabai -m signal --add label=chat-window-created event=window_created app='${chatApps}' action='${lib.getExe placeChat} "$YABAI_WINDOW_ID"'
      yabai -m signal --add label=chat-display-added event=display_added action='sleep ${chatPlacementDelay}; ${lib.getExe placeChat}'
      yabai -m signal --add label=chat-display-removed event=display_removed action='sleep ${chatPlacementDelay}; ${lib.getExe placeChat}'
      ${lib.getExe placeChat} &

      yabai -m signal --add label=dock-restart-display-added event=display_added action="sleep ${toString dockRestartDelay}; killall Dock"
      yabai -m signal --add label=dock-restart-display-removed event=display_removed action="sleep ${toString dockRestartDelay}; killall Dock"
    '';
  };

  launchd.user.agents.yabai.serviceConfig = {
    StandardOutPath = "/tmp/yabai_${user}.out.log";
    StandardErrorPath = "/tmp/yabai_${user}.err.log";
  };
}
