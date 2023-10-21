{ pkgs, user, ... }:

{
  imports = [
    ../../common/home
  ];

  home = {
    username = "${user}";
    homeDirectory = "/Users/${user}";
    stateVersion = "22.05";
    packages = with pkgs; [
      sox
      mediainfo
      statix
    ];
    file.".hushlogin" = {
      enable = true; # remove message when entering terminal
      text = "";
    };
  };

  common = {
    neovim.enable = true;
    firefox.enable = true;
    kitty.enable = true;
    git.enable = true;
    cli.enable = true;
    ssh.enable = true;
    tmux.enable = true;
    yazi.enable = true;
  };

  programs.home-manager.enable = true;
}
