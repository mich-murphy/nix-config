{
  lib,
  config,
  pkgs,
  ...
}:
# NOTE: Neovim config needs to be cloned manually:
# git clone git@github.com:mich-murphy/neovim.git ~/.config/nvim
# allows management of neovim outside of nix (for use on any computer)
with lib; let
  cfg = config.common.neovim;
in {
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
        alejandra
        nil
      ];
      extraPython3Packages = py:
        with py; [
          pip
        ];
    };

    home.sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
    };
  };
}
