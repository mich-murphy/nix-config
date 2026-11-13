{ pkgs, ... }:
{
  home.packages = [
    pkgs.azure-cli
    pkgs.doctl
  ];

  programs.neovim.extraPackages = [
    pkgs.ghostscript
    pkgs.imagemagick
    pkgs.mermaid-cli
    pkgs.nodejs
    pkgs.tectonic
  ];
}
