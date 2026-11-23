{ pkgs, ... }:
{
  home.packages = [
    pkgs.azure-cli
    pkgs.doctl
    pkgs.namespace-cli
    pkgs.mole-cleaner
  ];

  programs.neovim.extraPackages = [
    pkgs.ghostscript
    pkgs.imagemagick
    pkgs.mermaid-cli
    pkgs.nodejs
    pkgs.tectonic
  ];
}
