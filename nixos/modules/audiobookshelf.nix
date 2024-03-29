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
    domain = mkOption {
      type = types.str;
      default = "audiobookshelf.pve.elmurphy.com";
      description = "Domain for Audiobookshelf to be made available";
    };
    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address of Audiobookshelf host";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      audiobookshelf = {
        enable = true;
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.host}:${toString config.services.audiobookshelf.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    users.users.audiobookshelf.extraGroups = ["media"];
  };
}
