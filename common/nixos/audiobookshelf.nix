{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.audiobookshelf;
in
{
  options.common.audiobookshelf = {
    enable = mkEnableOption "Enable Audiobookshelf";
    workingDir = mkOption {
      type = types.str;
      default = "/var/lib/audiobookshelf";
      description = "Path to Audiobookshelf config files";
    };
    port = mkOption {
      type = types.str;
      default = "13378";
      description = "Port for linkding to be advertised on";
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
      containers."audiobookshelf" = {
        autoStart = true;
        image = "ghcr.io/advplyr/audiobookshelf:latest";
        environment = {
          AUDIOBOOKSHELF_UID = "99";
          AUDIOBOOKSHELF_GID = "100";
        };
        ports = [ "${cfg.port}:80" ];
        volumes = [
          "/data/media/audiobooks:/audiobooks"
          "/data/media/podcasts:/podcasts"
          "${cfg.workingDir}/config:/config"
          "${cfg.workingDir}/audiobooks:/metadata"
        ];
      };
    };

    services.nginx = mkIf cfg.nginx {
      virtualHosts."audiobookshelf.pve.elmurphy.com"= {
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
