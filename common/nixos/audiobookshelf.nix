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
        ports = [ "13378:80" ];
        volumes = [
          "/data/media/audiobooks:/audiobooks"
          "/data/media/podcasts:/podcasts"
          "${cfg.workingDir}/config:/config"
          "${cfg.workingDir}/audiobooks:/metadata"
        ];
      };
    };
  };
}
