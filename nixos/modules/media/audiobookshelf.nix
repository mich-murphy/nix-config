{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.audiobookshelf;
in {
  options.common.audiobookshelf = {
    enable = mkEnableOption "Enable Audiobookshelf";
    extraGroups = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Additional groups for audiobookshelf user";
      example = ["media"];
    };
    domain = mkOption {
      type = types.str;
      default = "audiobookshelf.pve.elmurphy.com";
      description = "Domain for Audiobookshelf";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address of Audiobookshelf host";
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
      audiobookshelf = {
        enable = true;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
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
