{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.lidarr;
in {
  options.common.lidarr = {
    enable = mkEnableOption "Enable Lidarr";
    group = mkOption {
      type = types.str;
      default = "lidarr";
      description = "Group for lidarr user";
      example = "media";
    };
    domain = mkOption {
      type = types.str;
      default = "lidarr.pve.elmurphy.com";
      description = "Domain for Lidarr";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP for Lidarr host";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
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
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:8686";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
