{
  lib,
  config,
  ...
}: let
  cfg = config.common.mealie;
in {
  options.common.mealie = {
    enable = lib.mkEnableOption "Enable mealie";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "mealie.pve.elmurphy.com";
      description = "Domain for mealie";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for mealie host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 9009;
      description = "Port for mealie";
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
      mealie = {
        enable = true;
        listenAddress = cfg.hostAddress;
        port = cfg.port;
        settings = {
          # ALLOW_SIGNUP = "false";
        };
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
