{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.spotifyd;
in {
  options.common.spotifyd = {
    enable = mkEnableOption "Add Spotifyd configuration for .config/";
  };

  config = mkIf cfg.enable {
    xdg.configFile."spotifyd/spotifyd.conf" = {
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
}
