{ pkgs, ... }:
{
  home.packages = [
    pkgs.azure-cli
    pkgs.doctl
    # tw93's Mole (`mo`); nixpkgs' `mole` is an unrelated SSH tunnel tool.
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
