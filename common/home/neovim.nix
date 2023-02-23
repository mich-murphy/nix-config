{ lib, config, pkgs, ... }:

# NOTE: Neovim Config Needs to Be Clones Manually:
# git clone git@github.com:mich-murphy/neovim.git ~/.config/nvim
# allows management of neovim outside of nix (for use on any computer)
 
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
      package = pkgs.neovim;
      vimAlias = true;
      defaultEditor = true;
      withPython3 = true;
      withNodeJs = true;
      extraPackages = with pkgs; [
        nodePackages.npm
        wget
        lazygit
        cargo
      ];
    };

    home.sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
    };
  };
}
