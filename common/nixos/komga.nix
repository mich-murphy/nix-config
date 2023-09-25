{ lib, config, ... }:

with lib;

let
  cfg = config.common.komga;
in
{
  options.common.komga = {
    enable = mkEnableOption "Enable Komga";
    port = mkOption {
      type = types.port;
      default = 6080;
      description = "Port for Komga to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      komga = {
        enable = true;
        inherit (cfg) port;
        openFirewall = true;
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."komga.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    users.users.komga.extraGroups = [ "media" ];
  };
 }
