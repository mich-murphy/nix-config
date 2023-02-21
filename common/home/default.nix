{ lib, config, pkgs, ... }:

{
  imports = [
    ./neovim
    ./firefox
    ./alacritty.nix
    ./kitty.nix
    ./git.nix
    ./cli.nix
    ./ssh.nix
    ./tmux.nix
    ./lf.nix
  ];
}
