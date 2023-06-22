{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.code-server;
in
{
  options.common.code-server = {
    enable = mkEnableOption "Enable Code Server";
    port = mkOption {
      type = types.port;
      default = 4444;
      description = "Port for Code Server to be advertised on";
    };
    host = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = "Host address for Code Server";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      code-server = {
        enable = true;
        package = pkgs.code-server;
        host = cfg.host;
        port = cfg.port;
        proxyDomain = "code.pve.elmurphy.com";
        disableTelemetry = true;
        disableUpdateCheck =true;
        auth = "none";
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${config.services.code-server.proxyDomain}= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.host}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    nixpkgs.config.permittedInsecurePackages = [
      "nodejs-16.20.0"
    ];
  };
 }
