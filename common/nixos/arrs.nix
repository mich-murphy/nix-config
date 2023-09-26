{ lib, config, ... }:

with lib;

let
  cfg = config.common.arrs;
in
{
  options.common.arrs = {
    enable = mkEnableOption "Enable arr services";
    enableNzbget = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable sabnzbd";
    };
    enableSonarr = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable sonarr";
    };
    enableRadarr = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable radarr";
    };
    enableLidarr = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable lidarr";
    };
    enableKapowarr = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable kapowarr";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    virtualisation.oci-containers = mkIf cfg.enableKapowarr {
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
      nzbget.enable = if cfg.enableNzbget then true else false; # default user: nzbget, default pass: tegbzn6789 
      sonarr.enable = if cfg.enableSonarr then true else false;
      radarr.enable = if cfg.enableRadarr then true else false;
      lidarr.enable = if cfg.enableLidarr then true else false;
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."nzbget.pve.elmurphy.com" = mkIf cfg.enableSabnzbd {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:6789";
            proxyWebsockets = true;
          };
        };
        virtualHosts."sonarr.pve.elmurphy.com" = mkIf cfg.enableSonarr {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8989";
            proxyWebsockets = true;
          };
        };
        virtualHosts."radarr.pve.elmurphy.com" = mkIf cfg.enableRadarr {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:7878";
            proxyWebsockets = true;
          };
        };
        virtualHosts."lidarr.pve.elmurphy.com" = mkIf cfg.enableLidarr {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8686";
            proxyWebsockets = true;
          };
        };
        virtualHosts."kapowarr.pve.elmurphy.com" = mkIf cfg.enableKapowarr {
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
