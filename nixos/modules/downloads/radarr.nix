{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.radarr;
in {
  options.common.radarr = {
    enable = mkEnableOption "Enable Radarr";
    group = mkOption {
      type = types.str;
      default = "radarr";
      description = "Group for radarr user";
      example = "media";
    };
    domain = mkOption {
      type = types.str;
      default = "radarr.pve.elmurphy.com";
      description = "Domain for radarr";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP for Radarr host";
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
      radarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:7878";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
