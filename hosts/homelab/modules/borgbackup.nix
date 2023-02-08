{ lib, config, pkgs, ... }:

let
  ssh-key = config.age.secrets.borgSSHKey.path;
in
{
  services.borgbackup.jobs = {
    "media" = {
      paths = [
        "/data/media/music"
        "/data/media/audiobooks"
        "/data/backup/RoonBackups"
      ];
      repo = "g268tdfo@g268tdfo.repo.borgbase.com:repo";
      encryption = {
        mode = "repokey-blake2";
        passCommand = "cat ${config.age.secrets.mediaBorgPass.path}";
      };
      environment.BORG_RSH = "ssh -i ${ssh-key}";
      compression = "auto,lzma";
      startAt = "daily";
    };
    "nextcloud" = {
      paths = [
        "/data/backup/nextclouddb"
        "/var/lib/nextcloud"
      ];
      repo = "duqvv98y@duqvv98y.repo.borgbase.com:repo";
      encryption = {
        mode = "repokey-blake2";
        passCommand = "cat ${config.age.secrets.nextcloudBorgPass.path}";
      };
      environment.BORG_RSH = "ssh -i ${ssh-key}";
      compression = "auto,lzma";
      startAt = "daily";
    };
  };
   
  age.secrets.mediaBorgPass.file = ../../../secrets/mediaBorgPass.age;
  age.secrets.nextcloudBorgPass.file = ../../../secrets/nextcloudBorgPass.age;
  age.secrets.borgSSHKey.file = ../../../secrets/borgSSHKey.age;
}
