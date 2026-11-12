{ pkgs, ... }:
{
  programs.neovim = {
    enable = true;
    vimAlias = true;
    defaultEditor = true;
    withPython3 = false;
    withRuby = false;
    sideloadInitLua = true;
    extraPackages = [
      pkgs.gnutar
      pkgs.go
      pkgs.gzip
      pkgs.nixfmt
      pkgs.nixd
      pkgs.python3
      pkgs.stdenv.cc
      pkgs.tree-sitter
      pkgs.unzip
    ];
  };
}
