{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.neovim;
in
{
  options.common.neovim = {
    enable = mkEnableOption "Enable neovim with personalised config";
  };

  config = mkIf cfg.enable {
    programs.neovim = {
      enable = true;
      package = pkgs.neovim-unwrapped;
      vimAlias = true;
      defaultEditor = true;
      withPython3 = true;
      withNodeJs = true;
      extraPackages = with pkgs; [
        nodePackages.npm
        wget
        lazygit
        cargo
        nixpkgs-fmt
        nixd
      ];
      extraPython3Packages = py: with py; [
        pip
      ];
    };

    home.sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
    };

    home.file.".config/nvim" = {
      source = ./nvim;
      recursive = true;
    };
  };
}
