{
  lib,
  config,
  ...
}: let
  cfg = config.common.audiobookshelf;
in {
  options.common.audiobookshelf = {
    enable = lib.mkEnableOption "Enable Audiobookshelf";
    extraGroups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Additional groups for audiobookshelf user";
      example = ["media"];
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "audiobookshelf.pve.elmurphy.com";
      description = "Domain for Audiobookshelf";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of Audiobookshelf host";
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
      audiobookshelf = {
        enable = true;
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString config.services.audiobookshelf.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    users.users.audiobookshelf.extraGroups = cfg.extraGroups;
  };
}
