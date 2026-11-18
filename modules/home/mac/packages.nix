{ pkgs, ... }:
{
  home.packages = [
    pkgs.azure-cli
    pkgs.doctl
    # tw93's Mole (`mo`); nixpkgs' `mole` is an unrelated SSH tunnel tool.
    pkgs.mole-cleaner
  ];

  # Spotify in the terminal. It plays through librespot and needs Premium;
  # `spotify_player authenticate` opens the browser to log in. Mac only: dev-01
  # has no audio.
  programs.spotify-player = {
    enable = true;
    settings = {
      # Our own Spotify app, so requests avoid the rate limits of ncspot's
      # shared client ID. Its redirect URI must be http://127.0.0.1:8989/login.
      # The absolute path is where the 1Password app's CLI integration finds
      # `op`.
      client_id_command = {
        command = "/usr/local/bin/op";
        args = [
          "read"
          "--no-newline"
          "--account"
          "xportal.1password.com"
          "op://Private/spotify_player/credential"
        ];
      };
      # By default playlist requests always use ncspot's shared, often
      # rate-limited, client ID: a Development Mode app gets other people's
      # playlists back with no tracks and no error. Our own playlists matter
      # more; copy someone else's to play it from here.
      ncspot_only_get_endpoints = [ ];
      # Media keys and Now Playing; off by default on macOS.
      enable_media_control = true;
      pause_on_startup = true;
      border_type = "Rounded";
      progress_bar_type = "Line";
      device = {
        name = "laptop";
        normalization = true;
      };
    };
    keymaps = [
      {
        command = "PageSelectNextOrScrollDown";
        key_sequence = "C-d";
      }
      {
        command = "PageSelectPreviousOrScrollUp";
        key_sequence = "C-u";
      }
    ];
    actions = [
      {
        action = "GoToArtist";
        key_sequence = "g A";
        target = "PlayingTrack";
      }
      {
        action = "GoToAlbum";
        key_sequence = "g B";
        target = "PlayingTrack";
      }
      {
        action = "ToggleLiked";
        key_sequence = "C-l";
      }
    ];
  };

  programs.neovim.extraPackages = [
    pkgs.ghostscript
    pkgs.imagemagick
    pkgs.mermaid-cli
    pkgs.nodejs
    pkgs.tectonic
  ];
}
