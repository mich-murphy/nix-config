{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.komga;
in {
  options.common.komga = {
    enable = mkEnableOption "Enable Komga";
    extraGroups = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Additional groups for komga user";
      example = ["media"];
    };
    domain = mkOption {
      type = types.str;
      default = "komga.pve.elmurphy.com";
      description = "Domain for Komga";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address of Komga host";
    };
    port = mkOption {
      type = types.port;
      default = 6080;
      description = "Port for Komga";
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
      komga = {
        enable = true;
        port = cfg.port;
        openFirewall = true;
      };
      nginx = mkIf cfg.nginx {
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

    users.users.komga.extraGroups = cfg.extraGroups;
  };
}
