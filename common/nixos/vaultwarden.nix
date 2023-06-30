{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.vaultwarden;
in
{
  options.common.vaultwarden = {
    enable = mkEnableOption "Enable Vaultwarden";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      vaultwarden = {
        enable = true;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts."vaultwarden.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8000";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
 }
