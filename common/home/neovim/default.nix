{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.neovim;
in
{
  options.common.neovim = {
    enable = mkEnableOption "Enable LazyVim with personalised config";
  };

  config = mkIf cfg.enable {
    programs.neovim = {
      enable = true;
      package = pkgs.neovim;
      viAlias = true;
      vimAlias = true;
      defaultEditor = true;
      withPython3 = true;
      withNodeJs = true;
      extraConfig = "luafile ~/.config/nvim/settings.lua";
      extraPackages = with pkgs; [
        nodePackages.npm
        wget
        # (python310.withPackages (ps: with ps; [
        #   black
        #   flake8
        #   debugpy
        # ]))
      ];
    };

    home.sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
    };

    xdg.configFile = {
      nvim = {
        source = ./nvim;
        target = "nvim";
        recursive = true;
      };
    };
  };
}
