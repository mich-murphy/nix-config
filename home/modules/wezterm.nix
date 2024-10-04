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
        local config = {}

        config.leader = { key = "a", mods = "CTRL", timeout_milliseconds = 2000 }
        config.keys = {
          {
            mods = "LEADER",
            key = "c",
            action = wezterm.action.SpawnTab("CurrentPaneDomain"),
          },
          {
            mods = "LEADER",
            key = "x",
            action = wezterm.action.CloseCurrentPane({confirm = true }),
          },
          {
            mods = "LEADER",
            key = "p",
            action = wezterm.action.ActivateTabRelative(-1),
          },
          {
            mods = "LEADER",
            key = "n",
            action = wezterm.action.ActivateTabRelative(1),
          },
          {
            mods = "LEADER",
            key = "|",
            action = wezterm.action.SplitHorizontal({ domain = "CurrentPaneDomain" }),
          },
          {
            mods = "LEADER",
            key = "-",
            action = wezterm.action.SplitVertical({ domain = "CurrentPaneDomain" }),
          },
          {
            mods = "CTRL",
            key = "h",
            action = wezterm.action.ActivatePaneDirection("Left"),
          },
          {
            mods = "CTRL",
            key = "j",
            action = wezterm.action.ActivatePaneDirection("Down"),
          },
          {
            mods = "CTRL",
            key = "k",
            action = wezterm.action.ActivatePaneDirection("Up"),
          },
          {
            mods = "CTRL",
            key = "l",
            action = wezterm.action.ActivatePaneDirection("Right"),
          },
          {
            mods = "CTRL",
            key = "LeftArrow",
            action = wezterm.action.AdjustPaneSize({ "Left", 5 }),
          },
          {
            mods = "CTRL",
            key = "RightArrow",
            action = wezterm.action.AdjustPaneSize({ "Right", 5 }),
          },
          {
            mods = "CTRL",
            key = "DownArrow",
            action = wezterm.action.AdjustPaneSize({ "Down", 5 }),
          },
          {
            mods = "CTRL",
            key = "UpArrow",
            action = wezterm.action.AdjustPaneSize({ "Up", 5 }),
          },
          {
            mods = "LEADER",
            key = "z",
            action = wezterm.action.TogglePaneZoomState,
          },
          {
            mods = "LEADER",
            key = "[",
            action = wezterm.action.ActivateCopyMode,
          },
          {
            mods = "LEADER",
            key = ",",
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
            mods = "LEADER",
            key = "w",
            action = wezterm.action.ShowTabNavigator,
          },
        }

        -- Leader + number to switch tab
        for i = 0, 9 do
          table.insert(config.keys, {
            mods = "LEADER",
            key = tostring(i),
            action = wezterm.action.ActivateTab(i),
          })
        end

        -- Tab bar
        config.hide_tab_bar_if_only_one_tab = false
        config.use_fancy_tab_bar = false
        config.tab_and_split_indices_are_zero_based = true

        -- General
        config.color_scheme = 'tokyonight_night'
        config.font = wezterm.font 'JetBrains Mono'
        config.font_size = 13.0
        config.use_dead_keys = false
        config.window_decorations = "RESIZE"
        config.adjust_window_size_when_changing_font_size = false
        config.window_close_confirmation = 'NeverPrompt'

        return config;
      '';
    };

    home.sessionVariables.TERMINAL = "wezterm";
  };
}
