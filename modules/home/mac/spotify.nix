{
  config,
  lib,
  osConfig ? null,
  pkgs,
  ...
}:
let
  op = [
    "/usr/local/bin/op"
    "read"
    "--no-newline"
    "--account"
    "xportal.1password.com"
  ];
  # The `spotify-soloist` cask in modules/darwin/applications.nix links it here.
  soloist = "/opt/homebrew/bin/soloist";
  # Reads the API key from 1Password and runs Soloist in the foreground,
  # forwarding launchd's stop signal so it shuts down cleanly. The key goes on
  # the command line because Soloist reads it nowhere else.
  soloistAgent = pkgs.writeShellApplication {
    name = "soloist-agent";
    text = ''
      if ! key=$(${lib.escapeShellArgs op} op://Private/spotify_player/soloist); then
        echo "soloist-agent: could not read the API key from 1Password; unlock it and run" \
          "launchctl kickstart -k gui/$(id -u)/org.nix-community.home.soloist" >&2
        exit 0
      fi
      ${soloist} --device-name laptop --api-key "$key" --ws 127.0.0.1:0 &
      pid=$!
      unset key
      trap 'kill -TERM "$pid" 2>/dev/null || true' TERM INT HUP
      status=0
      while :; do
        status=0
        wait "$pid" || status=$?
        kill -0 "$pid" 2>/dev/null || break
      done
      if [[ $status -eq 10 ]]; then
        echo "soloist-agent: this Soloist build has expired; switch to upgrade the cask, then run" \
          "launchctl kickstart -k gui/$(id -u)/org.nix-community.home.soloist" >&2
        exit 0
      fi
      exit "$status"
    '';
  };
in
{
  assertions = lib.optional (osConfig != null) {
    assertion = lib.any (cask: cask.name == "spotify-soloist") osConfig.homebrew.casks;
    message = "spotify.nix: the Soloist agent runs ${soloist}, which the missing spotify-soloist cask installs";
  };

  # Spotify Soloist, Spotify's own headless Connect player, plays the audio.
  # librespot, which spotify_player streams through, cannot get audio keys for
  # this account (librespot issue 1649). Pair it once from the Spotify app on
  # the same network; the session is then kept in its data directory.
  launchd.agents.soloist = {
    enable = true;
    config = {
      ProgramArguments = [ (lib.getExe soloistAgent) ];
      # Restart after a crash, but not after a clean exit: an expired build or
      # a locked 1Password would otherwise ask for approval on every restart.
      KeepAlive.SuccessfulExit = false;
      RunAtLoad = true;
      ThrottleInterval = 30;
      StandardOutPath = "${config.home.homeDirectory}/Library/Logs/soloist.log";
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/soloist.log";
    };
  };

  # spotify_player is the terminal remote for Soloist. Needs Premium;
  # `spotify_player authenticate` opens the browser to log in.
  programs.spotify-player = {
    enable = true;
    settings = {
      # Our own Spotify app, so requests avoid the rate limits of ncspot's
      # shared client ID. Its redirect URI must be http://127.0.0.1:8989/login.
      # The absolute path is where the 1Password app's CLI integration finds
      # `op`.
      client_id_command = {
        command = builtins.head op;
        args = builtins.tail op ++ [ "op://Private/spotify_player/credential" ];
      };
      # By default playlist requests always use ncspot's shared, often
      # rate-limited, client ID: a Development Mode app gets other people's
      # playlists back with no tracks and no error. Our own playlists matter
      # more; copy someone else's to play it from here.
      ncspot_only_get_endpoints = [ ];
      # Soloist plays; spotify_player's own librespot player fails every track
      # with `audio key error 0 1`.
      enable_streaming = "Never";
      # Media keys and Now Playing; off by default on macOS.
      enable_media_control = true;
      border_type = "Rounded";
      progress_bar_type = "Line";
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
}
