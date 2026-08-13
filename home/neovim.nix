{
  config,
  pkgs,
  repoRoot,
  ...
}: {
  programs.neovim = {
    enable = true;
    package = pkgs.neovim-unwrapped;
    vimAlias = true;
    defaultEditor = true;
    withPython3 = false;
    withNodeJs = false;
    withRuby = false;
    sideloadInitLua = true;
    extraPackages = [
      pkgs.alejandra
      pkgs.gnutar
      pkgs.gzip
      pkgs.ghostscript
      pkgs.imagemagick
      pkgs.mermaid-cli
      pkgs.nixd
      pkgs.stdenv.cc
      pkgs.tectonic
      pkgs.tree-sitter
      pkgs.unzip
    ];
  };

  xdg.configFile."nvim".source =
    config.lib.file.mkOutOfStoreSymlink "${repoRoot}/config/nvim";
}
