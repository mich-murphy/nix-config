{ lib, config, pkgs, ... }:

{
  imports = [
    ./neovim
    ./firefox
    ./alacritty.nix
    ./git.nix
    ./cli.nix
    ./ssh.nix
    ./tmux.nix
    ./lf.nix
  ];
}
