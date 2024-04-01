{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.prowlarr;
in {
  options.common.prowlarr = {
    enable = mkEnableOption "Enable Prowlarr";
    domain = mkOption {
      type = types.str;
      default = "prowlarr.pve.elmurphy.com";
      description = "Domain for Prowlarr";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP for Prowlarr host";
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
      # https://wiki.servarr.com/prowlarr/faq#help-i-have-locked-myself-out
      prowlarr.enable = true;
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:9696";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
