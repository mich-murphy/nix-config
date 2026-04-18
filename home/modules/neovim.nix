{
  lib,
  config,
  pkgs,
  ...
}:
# NOTE: Neovim config needs to be cloned manually:
# git clone git@github.com:mich-murphy/neovim.git ~/.config/nvim
# allows management of neovim outside of nix (for use on any computer)
let
  cfg = config.common.neovim;
in {
  options.common.neovim = {
    enable = lib.mkEnableOption "Enable neovim with personalised config";
  };

  config = lib.mkIf cfg.enable {
    programs.neovim = {
      enable = true;
      package = pkgs.neovim-unwrapped;
      vimAlias = true;
      defaultEditor = true;
      withPython3 = false;
      withNodeJs = false;
      withRuby = false;
      initLua = ''require("config.lazy")'';
      extraPackages = with pkgs; [
        wget
        lazygit
        cargo
        alejandra
        nixd
        nodejs
        go
        tree-sitter
        imagemagick
        ghostscript
        tectonic
        mermaid-cli
      ];
    };

    home.sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
    };
  };
}
