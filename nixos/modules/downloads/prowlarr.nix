{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.prowlarr;
in {
  options.common.prowlarr = {
    enable = lib.mkEnableOption "Enable Prowlarr";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "prowlarr.pve.elmurphy.com";
      description = "Domain for Prowlarr";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Prowlarr host";
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
      # https://wiki.servarr.com/prowlarr/faq#help-i-have-locked-myself-out
      prowlarr = {
        enable = true;
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:9696";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
