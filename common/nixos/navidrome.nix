{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.navidrome;
in
{
  options.common.navidrome = {
    enable = mkEnableOption "Enable Navidrome";
    port = mkOption {
      type = types.port;
      default = 4533;
      description = "Port for Navidrome to be advertised on";
    };
    host = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = "Host address for Navidrome";
    };
    musicFolder = mkOption {
      type = types.str;
      default = "/data/media/music";
      description = "Path to music on host";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      navidrome = {
        enable = true;
        settings = {
          Address = "${cfg.host}";
          Port = cfg.port;
          MusicFolder = "${cfg.musicFolder}";
        };
      };
      nginx = mkIf cfg.nginx {
        virtualHosts."navidrome.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.host}:${builtins.toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
 }
