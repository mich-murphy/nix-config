{ pkgs, ... }:
{
  # The configuration itself is the mich-murphy/neovim repository, cloned to
  # ~/.config/nvim; Nix supplies only Neovim and its runtime tools.
  programs.neovim = {
    enable = true;
    package = pkgs.neovim-unwrapped;
    vimAlias = true;
    defaultEditor = true;
    withPython3 = false;
    withRuby = false;
    sideloadInitLua = true;
    extraPackages = [
      pkgs.gnutar
      pkgs.gzip
      pkgs.nixfmt
      pkgs.nixd
      pkgs.stdenv.cc
      pkgs.tree-sitter
      pkgs.unzip
    ];
  };
}
