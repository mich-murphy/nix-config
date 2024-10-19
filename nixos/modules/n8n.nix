{
  lib,
  config,
  ...
}: let
  cfg = config.common.n8n;
in {
  options.common.n8n = {
    enable = lib.mkEnableOption "Enable n8n";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "n8n.pve.elmurphy.com";
      description = "Domain for n8n";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of n8n host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 5678;
      description = "Port for n8n";
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
      n8n = {
        enable = true;
        webhookUrl = "https://${cfg.domain}";
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
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
