{
  lib,
  config,
  ...
}: let
  cfg = config.common.komga;
in {
  options.common.komga = {
    enable = lib.mkEnableOption "Enable Komga";
    extraGroups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Additional groups for komga user";
      example = ["media"];
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "komga.pve.elmurphy.com";
      description = "Domain for Komga";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of Komga host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 6080;
      description = "Port for Komga";
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
      komga = {
        enable = true;
        port = cfg.port;
        openFirewall = true;
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

    users.users.komga.extraGroups = cfg.extraGroups;
  };
}
