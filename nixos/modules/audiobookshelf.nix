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
        virtualHosts."audiobookshelf.pve.elmurphy.com" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString config.services.audiobookshelf.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    users.users.audiobookshelf.extraGroups = ["media"];
  };
}
