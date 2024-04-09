{
  lib,
  pkgs,
  config,
  ...
}:
# reference: https://danth.github.io/stylix/options/hm.html
with lib; let
  cfg = config.common.stylix;
in {
  options.common.stylix = {
    enable = mkEnableOption "Enable Stylix to theme system";
  };

  config = mkIf cfg.enable {
    stylix = {
      image = ../../home/modules/stylix/wallpaper.jpg; # from host path
      polarity = "dark";
      base16Scheme = "${pkgs.base16-schemes}/share/themes/tokyo-night-dark.yaml";
      fonts = {
        monospace = {
          name = "JetBrainsMono Nerd Font";
          package = pkgs.nerdfonts.override {fonts = ["JetBrainsMono"];};
        };
        serif = config.stylix.fonts.monospace;
        sansSerif = config.stylix.fonts.monospace;
        emoji = config.stylix.fonts.monospace;
        sizes = {
          applications = 13;
          terminal = 13;
        };
      };
      targets = {
        vim.enable = false;
        kitty.variant256Colors = true;
        # bat.enable = true;
        # btop.enable = true;
        # fzf.enable = true;
        # kitty.enable = true;
        # lazygit.enable = true;
        # tmux.enable = true;
        # yazi.enable = true;
        # zellij.enable = true;
        firefox.profileNames = ["mm"];
      };
    };
  };
}
