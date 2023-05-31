{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.syncthing;
in
{
  options.common.syncthing = {
    enable = mkEnableOption "Enable syncthing with connection to seedbox";
  };

  config = mkIf cfg.enable {
    services = {
      syncthing = {
        enable = true;
        guiAddress = "0.0.0.0:8384";
        overrideDevices = true;
        overrideFolders = true;
        settings = {
          gui.insecureAdminAccess = true;
          devices = {
            "seedbox".id = "5N3E33W-SCXYEL5-URIJLAW-Y32VCKK-UYNSVR2-R5I6KMJ-YZ4CIKB-6SDUOAT";
          };
          folders = {
            "Music" = {
              id = "mrpfh-btugj";
              path = "/data/media/music";
              devices = [ "seedbox" ];
              type = "receiveonly";
            };
            "Audiobooks" = {
              id = "mqh32-k7ykn";
              path = "/data/media/audiobooks";
              devices = [ "seedbox" ];
              type = "receiveonly";
            };
            "Movies" = {
              id = "naolq-r7zlm";
              path = "/data/media/movies";
              devices = [ "seedbox" ];
              type = "receiveonly";
            };
            "TV" = {
              id = "hhqvi-jv4wy";
              path = "/data/media/tv";
              devices = [ "seedbox" ];
              type = "receiveonly";
            };
          };
        };
      };
    };
  };
}
