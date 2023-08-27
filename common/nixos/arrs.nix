{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.arrs;
in
{
  options.common.piracy = {
    enable = mkEnableOption "Enable arr services";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      sonarr.enable = true;
      radarr.enable = true;
      lidarr.enable = true;
      readarr.enable = true;
      bazarr.enable = true;
      prowlarr.enable = true;
      nginx = mkIf cfg.nginx {
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
        virtualHosts."bazarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:6767";
            proxyWebsockets = true;
          };
        };
        virtualHosts."prowlarr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:9696";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
