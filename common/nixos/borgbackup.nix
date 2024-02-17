{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.borgbackup;
in {
  options.common.borgbackup = {
    enable = mkEnableOption "Enable borgbackup for media and Nextcloud to BorgBase";
  };

  config = mkIf cfg.enable {
    services.borgbackup.jobs = {
      "media" = {
        paths = [
          "/var/lib/audiobookshelf"
          "/var/lib/freshrss"
          "/var/lib/gitea"
          "/var/lib/jellyfin"
          "/var/lib/komga"
          "/var/lib/lidarr"
          "/var/lib/nzbget"
          "/var/lib/prowlarr"
          "/var/lib/radarr"
          "/var/lib/readarr"
          "/var/lib/sonarr"
          "/var/lib/wallabag"
          "/var/lib/ytdlp-sub/ytdl-sub-configs"
        ];
        repo = "ssh://g268tdfo@g268tdfo.repo.borgbase.com/./repo";
        encryption = {
          mode = "repokey-blake2";
          passCommand = "cat ${config.age.secrets.mediaBorgPass.path}";
        };
        compression = "auto,lzma";
        startAt = "daily";
        prune.keep = {
          within = "1d";
          daily = 7;
          weekly = 4;
          monthly = -1;
        };
      };
      "nextcloud" = {
        paths = [
          "/data/backups/postgresql/nextcloud.sql.gz"
          "/data/nextcloud"
        ];
        repo = "ssh://duqvv98y@duqvv98y.repo.borgbase.com/./repo";
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
