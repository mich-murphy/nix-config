{
  lib,
  config,
  ...
}: let
  cfg = config.common.readarr;
in {
  options.common.readarr = {
    enable = lib.mkEnableOption "Enable Readarr";
    group = lib.mkOption {
      type = lib.types.str;
      default = "readarr";
      description = "Group for readarr user";
      example = "media";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "readarr.pve.elmurphy.com";
      description = "Domain for Readarr";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Readarr host";
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
      readarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:8787";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
