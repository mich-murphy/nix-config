# Tools only the Macs need: document and diagram rendering for Neovim, the
# DigitalOcean CLI, and Node for Mason (bootstrap pins Node on the Linux dev
# box).
{ pkgs, ... }:
{
  home.packages = [
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
