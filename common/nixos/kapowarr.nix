{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.kapowarr;
in
{
  options.common.kapowarr = {
    enable = mkEnableOption "Enable Kapowarr";
    port = mkOption {
      type = types.str;
      default = "5656";
      description = "Port for Kapowarr to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    virtualisation.oci-containers = {
      backend = "docker";
      containers."kapowarr" = {
        autoStart = true;
        image = "mrcas/kapowarr:latest";
        environment = {
          AUDIOBOOKSHELF_UID = "99";
          AUDIOBOOKSHELF_GID = "100";
        };
        ports = [ "${cfg.port}:5656" ];
        volumes = [
          "kapowarr-db:/app/db"
          "/data/temp/kapowarr:/app/temp_downloads"
          "/data/media/comics:/comics"
          "/data/media/manga:/manga"
        ];
      };
    };

    services.nginx = mkIf cfg.nginx {
      virtualHosts."kapowarr.pve.elmurphy.com"= {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${cfg.port}";
          proxyWebsockets = true;
        };
      };
    };
  };
 }
