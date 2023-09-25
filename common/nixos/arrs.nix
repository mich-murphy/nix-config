{ lib, config, ... }:

with lib;

let
  cfg = config.common.arrs;
in
{
  options.common.arrs = {
    enable = mkEnableOption "Enable arr services";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    virtualisation.oci-containers = {
      backend = "docker";
      containers."kapowarr" = {
        autoStart = true;
        image = "mrcas/kapowarr:latest";
        environment = {
          AUDIOBOOKSHELF_UID = "99";
          AUDIOBOOKSHELF_GID = "100";
        };
        ports = [ "5656:5656" ];
        volumes = [
          "kapowarr-db:/app/db"
          "/data/temp/kapowarr:/app/temp_downloads"
          "/data/media/comics:/comics"
          "/data/media/manga:/manga"
        ];
      };
    };
    services = {
      sonarr.enable = true;
      radarr.enable = true;
      lidarr.enable = true;
      readarr.enable = true;
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."sonarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8989";
            proxyWebsockets = true;
          };
        };
        virtualHosts."radarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:7878";
            proxyWebsockets = true;
          };
        };
        virtualHosts."lidarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8686";
            proxyWebsockets = true;
          };
        };
        virtualHosts."readarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8787";
            proxyWebsockets = true;
          };
        };
        virtualHosts."kapowarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:5656";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
