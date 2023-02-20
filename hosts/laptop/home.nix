{ config, pkgs, user, ... }:

{
  imports = [
    ../../common/home
  ];

  home = {
    username = "${user}";
    homeDirectory = "/Users/${user}";
    stateVersion = "22.05";
  };

  common = {
    neovim.enable = true;
    firefox.enable = true;
    alacritty.enable = true;
    git.enable = true;
    cli.enable = true;
    ssh.enable = true;
    tmux.enable = true;
    lf.enable = true;
  };

  programs.home-manager.enable = true;
}
