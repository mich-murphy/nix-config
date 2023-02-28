{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.alacritty;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in
{
  options.common.alacritty = {
    enable = mkEnableOption "Enable Alacritty with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.alacritty = {
      enable = true;
      package = fakepkg "alacritty";
      settings = {
        env = {
          TERM = "xterm-256color";
        };
        live_config_reload = true;
        dynamic_title = true;
        window = {
          decorations = "buttonless";
          padding = {
            x = 15;
            y = 15;
          };
        };
        font = {
          size = 13.0; 
          normal = {
            family = "JetBrainsMono Nerd Font";
            style = "Regular";
          };
          bold = {
            family = "JetBrainsMono Nerd Font";
            style = "Bold";
          };
          italic = {
            family = "JetBrainsMono Nerd Font";
            style = "Italic";
          };
        };
        draw_bold_text_with_bright_colors = true;
        colors = {
          primary = {
            background = "#1E1E2E";
            foreground = "#CDD6F4";
            dim_foreground = "#CDD6F4";
            bright_foreground = "#CDD6F4";
          };
          cursor = {
            text = "#1E1E2E";
            cursor = "#F5E0DC";
          };
          vi_mode_cursor = {
            text = "#1E1E2E";
            cursor = "#B4BEFE";
          };
          search = {
            matches = {
              foreground = "#1E1E2E";
              background = "#A6ADC8";
            };
            focused_match = {
              foreground = "#1E1E2E";
              background = "#A6E3A1";
            };
            footer_bar = {
              foreground = "#1E1E2E";
              background = "#A6ADC8";
            };
          };
          hints = {
            start = {
              foreground = "#1E1E2E";
              background = "#F9E2AF";
            };
            end = {
              foreground = "#1E1E2E";
              background = "#A6ADC8";
            };
          };
          selection = {
            text = "#1E1E2E";
            background = "#F5E0DC";
          };
          normal = {
            black = "#45475A";
            red = "#F38BA8";
            green = "#A6E3A1";
            yellow = "#F9E2AF";
            blue = "#89B4FA";
            magenta = "#F5C2E7";
            cyan = "#94E2D5";
            white = "#BAC2DE";
          };
          bright = {
            black = "#585B70";
            red = "#F38BA8";
            green = "#A6E3A1";
            yellow = "#F9E2AF";
            blue = "#89B4FA";
            magenta = "#F5C2E7";
            cyan = "#94E2D5";
            white = "#A6ADC8";
          };
          dim = {
            black = "#45475A";
            red = "#F38BA8";
            green = "#A6E3A1";
            yellow = "#F9E2AF";
            blue = "#89B4FA";
            magenta = "#F5C2E7";
            cyan = "#94E2D5";
            white = "#BAC2DE";
          };
          indexed_colors = [
            { index = 16; color = "#FAB387"; }
            { index = 17; color = "#F5E0DC"; }
          ];
        };
      };
    };

    home.sessionVariables.TERMINAL = "alacritty";
  };
}
