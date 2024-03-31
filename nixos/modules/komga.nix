{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.komga;
in {
  options.common.komga = {
    enable = mkEnableOption "Enable Komga";
    hostname = mkOption {
      type = types.str;
      default = "komga.pve.elmurphy.com";
      description = "Hostname for Komga";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address of Komga host";
    };
    port = mkOption {
      type = types.port;
      default = 6080;
      description = "Port for Komga";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      komga = {
        enable = true;
        port = cfg.port;
        openFirewall = true;
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts.${cfg.hostname} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    users.users.komga.extraGroups = ["media"];
  };
}
