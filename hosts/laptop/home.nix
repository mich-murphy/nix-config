{ pkgs, ... }:

let
  user = "mm";
in 
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
      _1password
    ];
    file = {
      # remove message when entering terminal
      ".hushlogin" = {
        enable = true;
        text = "";
      };
      ".hammerspoon" = {
        enable = true;
        recursive = true;
        source = ./hammerspoon;
      };
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

  xdg.configFile = {
    "spotifyd/spotifyd.conf" = {
      enable = true;
      target = "spotifyd/spotifyd.conf";
      text = ''
        [global]
        username = "spotify@elmurphy.com"
        password_cmd = "op read op://Private/spotify/password"
        backend = "portaudio"
        device_name = "macbook"
        device_type = "computer"
        no_audio_cache = true
        bitrate = 320
        volume_normalisation = true
        normalisation_pregain = -10
        autoplay = true
        volume_controller = "softvol"
        zeroconf_port = 1234
      '';
    };
  };

  programs.home-manager.enable = true;
}
