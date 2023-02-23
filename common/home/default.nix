{ lib, config, pkgs, ... }:

{
  imports = [
    ./firefox
    ./neovim.nix
    ./alacritty.nix
    ./kitty.nix
    ./git.nix
    ./cli.nix
    ./ssh.nix
    ./tmux.nix
    ./lf.nix
  ];
}
