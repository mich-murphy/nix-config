{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.sonarr;
in {
  options.common.sonarr = {
    enable = mkEnableOption "Enable Sonarr";
    group = mkOption {
      type = types.str;
      default = "sonarr";
      description = "Group for sonarr user";
      example = "media";
    };
    domain = mkOption {
      type = types.str;
      default = "sonarr.pve.elmurphy.com";
      description = "Domain for Sonarr";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP for Sonarr host";
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
      sonarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:8989";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
