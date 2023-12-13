{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.arrs;
in {
  options.common.arrs = {
    enable = mkEnableOption "Enable arr services";
    enableNzbget = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable sabnzbd";
    };
    enableProwlarr = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable prowlarr";
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
    enableReadarr = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable readarr";
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
        ports = ["5656:5656"];
        volumes = [
          "kapowarr-db:/app/db"
          "/tmp/kapowarr:/app/temp_downloads"
          "/mnt/data/media/comics:/comics"
          "/mnt/data/media/manga:/manga"
        ];
      };
    };
    services = {
      # https://wiki.servarr.com/prowlarr/faq#help-i-have-locked-myself-out
      prowlarr.enable =
        if cfg.enableProwlarr
        then true
        else false;
      nzbget = mkIf cfg.enableNzbget {
        # default user: nzbget, default pass: tegbzn6789
        enable = true;
        group = "media";
      };
      sonarr = mkIf cfg.enableSonarr {
        enable = true;
        group = "media";
      };
      radarr = mkIf cfg.enableRadarr {
        enable = true;
        group = "media";
      };
      lidarr = mkIf cfg.enableLidarr {
        enable = true;
        group = "media";
      };
      readarr = mkIf cfg.enableReadarr {
        enable = true;
        group = "media";
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."nzbget.pve.elmurphy.com" = mkIf cfg.enableNzbget {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:6789";
            proxyWebsockets = true;
          };
        };
        virtualHosts."prowlarr.pve.elmurphy.com" = mkIf cfg.enableProwlarr {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:9696";
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
        virtualHosts."readarr.pve.elmurphy.com" = mkIf cfg.enableLidarr {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8787";
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
