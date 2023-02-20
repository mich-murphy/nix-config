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
            background = "#222222";
            foreground = "#f7f1ff";
          };
          selection = {
            text = "#bab6c0";
            background = "#403e41";
          };
          normal = {
            black = "#363537";
            red = "#fc618d";
            green = "#7db88f";
            yellow = "#fce566";
            blue = "0x61afef";
            magenta = "#948ae3";
            cyan = "#5ad4e6";
            white = "#f7f1ff";
          }; 
          bright = {
            black = "#403e41";
            red = "#fc618d";
            green = "#7db88f";
            yellow = "#fce566";
            blue = "0x61afef";
            magenta = "#948ae3";
            cyan = "#5ad4e6";
            white = "#f7f1ff";
          }; 
        };
      };
    };

    home.sessionVariables.TERMINAL = "alacritty";
  };
}
