# Tools only the Macs need: document and diagram rendering for Neovim, the
# DigitalOcean CLI, and Node (bootstrap pins Node on the Linux dev box).
{ pkgs, ... }:
{
  home.packages = [
    pkgs.doctl
    pkgs.nodejs
  ];

  programs.neovim.extraPackages = [
    pkgs.ghostscript
    pkgs.imagemagick
    pkgs.mermaid-cli
    pkgs.tectonic
  ];
}
