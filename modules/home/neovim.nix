{ pkgs, ... }:
{
  programs.neovim = {
    enable = true;
    package = pkgs.neovim-unwrapped;
    vimAlias = true;
    defaultEditor = true;
    withPython3 = false;
    withRuby = false;
    sideloadInitLua = true;
    # Mason builds and runs its language servers and formatters with go,
    # python3 and (on the Macs) node, so they live here, not on the shell PATH.
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
