{ lib, config, ... }:

with lib;

let
  cfg = config.common.syncthing;
in
{
  options.common.syncthing = {
    enable = mkEnableOption "Enable syncthing with connection to seedbox";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      syncthing = {
        enable = true;
        group = "media";
        guiAddress = "0.0.0.0:8384";
        overrideDevices = true;
        overrideFolders = true;
        settings = {
          devices = {
            "seedbox".id = "5N3E33W-SCXYEL5-URIJLAW-Y32VCKK-UYNSVR2-R5I6KMJ-YZ4CIKB-6SDUOAT";
          };
          folders = {
            "Music" = {
              id = "mrpfh-btugj";
              path = "/data/media/music";
              devices = [ "seedbox" ];
              ignorePerms = true;
            };
            "Audiobooks" = {
              id = "mqh32-k7ykn";
              path = "/data/media/audiobooks";
              devices = [ "seedbox" ];
              ignorePerms = true;
            };
            "Movies" = {
              id = "naolq-r7zlm";
              path = "/data/media/movies";
              devices = [ "seedbox" ];
              ignorePerms = true;
            };
            "TV" = {
              id = "hhqvi-jv4wy";
              path = "/data/media/tv";
              devices = [ "seedbox" ];
              ignorePerms = true;
            };
          };
        };
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."syncthing.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${config.services.syncthing.guiAddress}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
