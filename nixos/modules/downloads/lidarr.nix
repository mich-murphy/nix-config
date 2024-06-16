{
  lib,
  config,
  ...
}: let
  cfg = config.common.lidarr;
in {
  options.common.lidarr = {
    enable = lib.mkEnableOption "Enable Lidarr";
    group = lib.mkOption {
      type = lib.types.str;
      default = "lidarr";
      description = "Group for lidarr user";
      example = "media";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "lidarr.pve.elmurphy.com";
      description = "Domain for Lidarr";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Lidarr host";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    services = {
      lidarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:8686";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
