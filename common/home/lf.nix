{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.lf;
in
{
  options.common.lf = {
    enable = mkEnableOption "Enable Lf with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.lf = {
      enable = true;
      keybindings = {
        DD = "delete";
        p = "paste";
        x = "cut";
        y = "copy";
        l = "open";
        c = "clear";
        gn = "cd ~/nix-config";
        gg = "cd ~/git";
        gd = "cd ~/Downloads";
        gD = "cd ~/Documents";
        gp = "cd ~/Pictures";
        gc = "cd ~/.config";
      };
      settings = {
        hidden = true;
        dirfirst = true;
        relativenumber = true;
        ignorecase = true;
        globsearch = true;
        scrolloff = 8;
      };
    };
  };
}
