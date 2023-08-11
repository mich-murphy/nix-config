{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.invidious;
in
{
  options.common.invidious = {
    enable = mkEnableOption "Enable Invidious service";
    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port for Invidious to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      invidious = {
        enable = true;
        #domain = "invidious.pve.elmurphy.com";
        port = cfg.port;
        #nginx.enable = mkIf (cfg.nginx) true;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${config.services.indivious.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
 }
