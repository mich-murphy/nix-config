{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.readarr;
in {
  options.common.readarr = {
    enable = mkEnableOption "Enable Readarr";
    group = mkOption {
      type = types.str;
      default = "readarr";
      description = "Group for readarr user";
      example = "media";
    };
    domain = mkOption {
      type = types.str;
      default = "readarr.pve.elmurphy.com";
      description = "Domain for Readarr";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP for Readarr host";
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
      readarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:8787";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
