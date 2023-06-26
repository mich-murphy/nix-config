{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.uptime-kuma;
in
{
  options.common.uptime-kuma = {
    enable = mkEnableOption "Enable Uptime Kuma";
    port = mkOption {
      type = types.port;
      default = 3001;
      description = "Port for Uptime Kuma to be advertised on";
    };
    host = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = "Host address for Uptime Kuma";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      uptime-kuma = {
        enable = true;
        settings = {
          HOST = "${cfg.host}";
          PORT = "${toString cfg.port}";
        };
      };
      nginx = mkIf cfg.nginx {
        virtualHosts."uptime-kuma.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.host}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
 }
