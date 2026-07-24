{pkgs, ...}:
# NOTE: Neovim config needs to be cloned manually:
# git clone git@github.com:mich-murphy/neovim.git ~/.config/nvim
# allows management of neovim outside of nix (for use on any computer)
{
  programs.neovim = {
    enable = true;
    package = pkgs.neovim-unwrapped;
    vimAlias = true;
    defaultEditor = true;
    withPython3 = false;
    withNodeJs = false;
    withRuby = false;
    initLua = ''require("config.lazy")'';
    extraPackages = [
      pkgs.wget
      pkgs.lazygit
      pkgs.cargo
      pkgs.alejandra
      pkgs.nixd
      pkgs.nodejs
      pkgs.go
      pkgs.tree-sitter
      pkgs.imagemagick
      pkgs.ghostscript
      pkgs.tectonic
      pkgs.mermaid-cli
    ];
  };

  home.sessionVariables = {
    EDITOR = "nvim";
    VISUAL = "nvim";
  };
}
