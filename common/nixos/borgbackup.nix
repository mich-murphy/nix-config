{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.borgbackup;
in
{
  options.common.borgbackup = {
    enable = mkEnableOption "Enable borgbackup for media and Nextcloud to BorgBase";
  };

  config = mkIf cfg.enable {
    services.borgbackup.jobs = {
      "media" = {
        paths = [
          "/data/media/music"
          "/data/media/books"
          "/data/media/comics"
          "/data/media/manga"
          "/data/media/audiobooks"
          "/data/backups/RoonBackups"
        ];
        repo = "g268tdfo@g268tdfo.repo.borgbase.com:repo";
        encryption = {
          mode = "repokey-blake2";
          passCommand = "cat ${config.age.secrets.mediaBorgPass.path}";
        };
        compression = "auto,lzma";
        startAt = "hourly";
        prune.keep = {
          within = "1d";
          daily = 7;
          weekly = 4;
          monthly = -1;
        };
      };
      "nextcloud" = {
        paths = [
          "/data/backups/nextclouddb"
          "/data/nextcloud"
        ];
        repo = "duqvv98y@duqvv98y.repo.borgbase.com:repo";
        encryption = {
          mode = "repokey-blake2";
          passCommand = "cat ${config.age.secrets.nextcloudBorgPass.path}";
        };
        # preHook = "/run/current-system/sw/bin/nextcloud-occ maintenance:mode --on";
        # postHook = "/run/current-system/sw/bin/nextcloud-occ maintenance:mode --off";
        compression = "auto,lzma";
        startAt = "daily";
        prune.keep = {
          within = "1d";
          daily = 7;
          weekly = 4;
          monthly = -1;
        };
      };
    };

    age.secrets.mediaBorgPass.file = ../../secrets/mediaBorgPass.age;
    age.secrets.nextcloudBorgPass.file = ../../secrets/nextcloudBorgPass.age;
  };
}
