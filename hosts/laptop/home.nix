{pkgs, ...}: {
  imports = [
    ../../common/home
  ];

  home = {
    username = "mm";
    homeDirectory = "/Users/mm";
    stateVersion = "22.05";
    packages = with pkgs; [
      sox
      mediainfo
      statix
      _1password
      pipx
      duckdb
    ];
    file = {
      # remove message when entering terminal
      ".hushlogin" = {
        enable = true;
        text = "";
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
    # allow unfree packages via CLI
    "nixpkgs/config.nix" = {
      enable = true;
      target = "nixpkgs/config.nix";
      text = ''
        {
          allowUnfree = true;
        }
      '';
    };
    "karabiner" = {
      enable = true;
      recursive = true;
      source = ./karabiner;
    };
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
