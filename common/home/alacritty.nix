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
            background = "0x1a1b26";
            foreground = "0xc0caf5";
          };
          normal = {
            black = "0x15161e";
            red = "0xf7768e";
            green = "#7db88f";
            yellow = "0x9ece6a";
            blue = "0x7aa2f7";
            magenta = "0xbb9af7";
            cyan = "0x7dcfff";
            white = "0xa9b1d6";
          }; 
          bright = {
            black = "0x414868";
            red = "0xf7768e";
            green = "0x9ece6a";
            yellow = "0xe0af68";
            blue = "0x7aa2f7";
            magenta = "0xbb9af7";
            cyan = "0x7dcfff";
            white = "0xc0caf5";
          }; 
          indexed_colors = [
            {
              index = 16;
              color = "0xff9e64";
            }
            { 
              index = 17;
              color = "0xdb4b4b";
            }
          ];
        };
      };
    };

    home.sessionVariables.TERMINAL = "alacritty";
  };
}
