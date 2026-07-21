{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.wezterm;
in {
  options.common.wezterm = {
    enable = lib.mkEnableOption "Enable Wezterm with personalised settings";
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.programs.kitty.enable == false;
        message = "Deactivate Kitty. Only one terminal emulator should be enabled";
      }
      {
        assertion = config.programs.alacritty.enable == false;
        message = "Deactivate Alacritty. Only one terminal emulator should be enabled";
      }
    ];
    programs.wezterm = {
      enable = true;
      enableZshIntegration = false;
      package = pkgs.fakepkg "wezterm";
      extraConfig = ''
        local smart_splits = wezterm.plugin.require("https://github.com/mrjones2014/smart-splits.nvim")
        local config = wezterm.config_builder()

        -- Show which key table is active in the status area
        wezterm.on("update-right-status", function(window, pane)
          local name = window:active_key_table()
          if name then
            name = "TABLE: " .. name
          end
          window:set_right_status(name or "")
        end)

        config.leader = { key = "a", mods = "CTRL", timeout_milliseconds = 2000 }
        config.keys = {
          { key = "h", mods = "CTRL", action = wezterm.action.ActivatePaneDirection("Left") },
          { key = "j", mods = "CTRL", action = wezterm.action.ActivatePaneDirection("Down") },
          { key = "k", mods = "CTRL", action = wezterm.action.ActivatePaneDirection("Up") },
          { key = "l", mods = "CTRL", action = wezterm.action.ActivatePaneDirection("Right") },
          { key = "c", mods = "LEADER", action = wezterm.action.SpawnTab("CurrentPaneDomain") },
          { key = "x", mods = "LEADER", action = wezterm.action.CloseCurrentPane({confirm = false }) },
          { key = "p", mods = "LEADER", action = wezterm.action.ActivateTabRelative(-1) },
          { key = "n", mods = "LEADER", action = wezterm.action.ActivateTabRelative(1) },
          { key = "|", mods = "LEADER", action = wezterm.action.SplitHorizontal({ domain = "CurrentPaneDomain" }) },
          { key = "-", mods = "LEADER", action = wezterm.action.SplitVertical({ domain = "CurrentPaneDomain" }) },
          { key = "z", mods = "LEADER", action = wezterm.action.TogglePaneZoomState },
          { key = "[", mods = "LEADER", action = wezterm.action.ActivateCopyMode },
          { key = " ", mods = "LEADER", action = wezterm.action.QuickSelect },
          { key = "w", mods = "LEADER", action = wezterm.action.ShowTabNavigator },
          {
            key = 'L',
            mods = 'CTRL|SHIFT',
            action = wezterm.action.Multiple {
              wezterm.action.ClearScrollback 'ScrollbackAndViewport',
              wezterm.action.SendKey { key = 'L', mods = 'CTRL' },
            },
          },
          {
            key = ",",
            mods = "LEADER",
            action = wezterm.action.PromptInputLine({
              description = "Enter name for tab",
              action = wezterm.action_callback(function(window, pane, line)
                if line then
                  window:active_tab():set_title(line)
                end
              end),
            }),
          },
          {
            key = "r",
            mods = "LEADER",
            action = wezterm.action.ActivateKeyTable {
              name = "resize_pane",
              one_shot = false,
            },
          },
        }

        config.key_tables = {
          -- Defines the keys that are active in our resize-pane mode.
          resize_pane = {
            { key = "h", action = wezterm.action.AdjustPaneSize { "Left", 1 } },
            { key = "l", action = wezterm.action.AdjustPaneSize { "Right", 1 } },
            { key = "k", action = wezterm.action.AdjustPaneSize { "Up", 1 } },
            { key = "j", action = wezterm.action.AdjustPaneSize { "Down", 1 } },
            -- Cancel the mode by pressing escape
            { key = "Escape", action = "PopKeyTable" },
          },
        }

        -- Leader + number to switch tab
        for i = 1, 8 do
          table.insert(config.keys, {
            key = tostring(i),
            mods = "LEADER",
            action = wezterm.action.ActivateTab(i - 1),
          })
        end

        -- Tab bar
        config.hide_tab_bar_if_only_one_tab = false
        config.use_fancy_tab_bar = false
        config.switch_to_last_active_tab_when_closing_tab = true

        -- Preserve Codex and Claude's activity indicator after a tab rename,
        -- while replacing the session name that follows it.
        wezterm.on("format-tab-title", function(tab)
          local pane = tab.active_pane
          local tab_title = tab.tab_title
          local pane_title = pane.title
          local title = pane_title

          if tab_title and #tab_title > 0 then
            title = tab_title

            if pane_title and #pane_title > 0 then
              local process_path = pane.foreground_process_name or ""
              local process_name = process_path:match("([^/]+)$") or process_path
              local is_ai_session = process_name:match("^codex") or process_name:match("^claude")

              if is_ai_session then
                -- Match one non-ASCII UTF-8 glyph followed by whitespace. This
                -- covers both tools' indicators without coupling to their frames.
                local activity_indicator = pane_title:match("^([\194-\244][\128-\191]*)%s+")
                if activity_indicator then
                  title = activity_indicator .. " " .. tab_title
                end
              end
            end
          end

          -- A custom title also bypasses WezTerm's default numeric prefix, so
          -- restore it using the same one-based numbering as the leader keys.
          return " " .. (tab.tab_index + 1) .. ": " .. (title or "") .. " "
        end)

        -- General
        config.color_scheme = 'tokyonight_night'
        config.font = wezterm.font 'Berkeley Mono'
        config.font_size = 13.0
        config.use_dead_keys = false
        config.send_composed_key_when_left_alt_is_pressed = false
        config.send_composed_key_when_right_alt_is_pressed = false
        config.window_decorations = "RESIZE"
        config.adjust_window_size_when_changing_font_size = false
        config.window_close_confirmation = 'NeverPrompt'
        config.enable_kitty_keyboard = false

        smart_splits.apply_to_config(config, {
          direction_keys = { "h", "j", "k", "l" },
          modifiers = {
            move = "CTRL",
          },
        })

        return config;
      '';
    };
  };
}
