{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.wezterm;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
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
      package = fakepkg "wezterm";
      extraConfig = ''
        local smart_splits = wezterm.plugin.require("https://github.com/mrjones2014/smart-splits.nvim")
        local config = wezterm.config_builder()

        wezterm.on("gui-startup", function(cmd)
          local mux = wezterm.mux

          -- allow commands passed via `wezterm start --` to be used
          local args = {}
          if cmd then
            args = cmd.args
          end

          local nix_tab, pane, window = mux.spawn_window({
            workspace = "develop",
            cwd = "/Users/mm/dev/nix-config/",
            args = args,
          })
          nix_tab:set_title("nix")

          local dev_tab, dev_pane = window:spawn_tab({
            cwd = "/Users/mm/dev/",
          })
          dev_tab:set_title("dev")

          --[[
          local terminal_pane = dev_pane:split({
            direction = "Right",
            size = 0.3,
          })
          ]]

          mux.set_active_workspace("develop")
        end)

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
          { key = "x", mods = "LEADER", action = wezterm.action.CloseCurrentPane({confirm = true }) },
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

        -- General
        config.color_scheme = 'carbonfox'
        config.font = wezterm.font 'Berkeley Mono'
        config.font_size = 13.0
        config.use_dead_keys = false
        config.window_decorations = "RESIZE"
        config.adjust_window_size_when_changing_font_size = false
        config.window_close_confirmation = 'NeverPrompt'

        smart_splits.apply_to_config(config, {
          direction_keys = { "h", "j", "k", "l" },
          modifiers = {
            move = "CTRL",
          },
        })

        return config;
      '';
    };

    xdg.configFile."wezterm/colors/carbonfox.toml" = {
      enable = true;
      force = true;
      text = ''
        [metadata]
        name = "carbonfox"
        author = "EdenEast"
        origin_url = "https://github.com/EdenEast/nightfox.nvim"

        [colors]
        foreground = "#f2f4f8"
        background = "#161616"
        cursor_bg = "#f2f4f8"
        cursor_border = "#f2f4f8"
        cursor_fg = "#161616"
        compose_cursor = '#3ddbd9'
        selection_bg = "#2a2a2a"
        selection_fg = "#f2f4f8"
        scrollbar_thumb = "#7b7c7e"
        split = "#0c0c0c"
        visual_bell = "#f2f4f8"
        ansi = ["#282828", "#ee5396", "#25be6a", "#08bdba", "#78a9ff", "#be95ff", "#33b1ff", "#dfdfe0"]
        brights = ["#484848", "#f16da6", "#46c880", "#2dc7c4", "#8cb6ff", "#c8a5ff", "#52bdff", "#e4e4e5"]

        [colors.indexed]
        16 = "#ff7eb6"
        17 = "#3ddbd9"

        [colors.tab_bar]
        background = "#0c0c0c"
        inactive_tab_edge = "#0c0c0c"
        inactive_tab_edge_hover = "#252525"

        [colors.tab_bar.active_tab]
        bg_color = "#7b7c7e"
        fg_color = "#161616"
        intensity = "Normal"
        italic = false
        strikethrough = false
        underline = "None"

        [colors.tab_bar.inactive_tab]
        bg_color = "#252525"
        fg_color = "#b6b8bb"
        intensity = "Normal"
        italic = false
        strikethrough = false
        underline = "None"

        [colors.tab_bar.inactive_tab_hover]
        bg_color = "#353535"
        fg_color = "#f2f4f8"
        intensity = "Normal"
        italic = false
        strikethrough = false
        underline = "None"

        [colors.tab_bar.new_tab]
        bg_color = "#161616"
        fg_color = "#b6b8bb"
        intensity = "Normal"
        italic = false
        strikethrough = false
        underline = "None"

        [colors.tab_bar.new_tab_hover]
        bg_color = "#353535"
        fg_color = "#f2f4f8"
        intensity = "Normal"
        italic = false
        strikethrough = false
        underline = "None"
      '';
    };

    home.sessionVariables.TERMINAL = "wezterm";
  };
}
