{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.tandoor;
in
{
  options.common.tandoor = {
    enable = mkEnableOption "Enable Tandoor";
    port = mkOption {
      type = types.port;
      default = 9010;
      description = "Port for Tandoor to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      tandoor-recipes = {
        enable = true;
        port = cfg.port;
        address = "127.0.0.1";
      };
      nginx = mkIf cfg.nginx {
        virtualHosts."tandoor.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${config.services.tandoor-recipes.address}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
 }
