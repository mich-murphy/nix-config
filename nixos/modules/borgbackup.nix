{
  lib,
  config,
  ...
}: let
  cfg = config.common.borgbackup;
in {
  options.common.borgbackup = {
    enable = lib.mkEnableOption "Enable borgbackup for specified filepaths";
    name = lib.mkOption {
      type = lib.types.str;
      default = config.networking.hostName;
      description = "Name of Borgbackup job for reference by Systemd";
      example = "media";
    };
    borgRepo = lib.mkOption {
      type = lib.types.str;
      description = "SSH URL of Borg repository";
      example = "ssh://g268tdfo@g268tdfo.repo.borgbase.com/./repo";
    };
    backupFrequency = lib.mkOption {
      type = lib.types.enum ["daily" "weekly" "monthly" "yearly"];
      default = "daily";
      description = "Frequency for files and folders to be backed up";
    };
    backupPaths = lib.mkOption {
      type = lib.types.nullOr (lib.types.coercedTo lib.types.str lib.singleton (lib.types.listOf lib.types.str));
      default = null;
      description = "List of backup filepaths";
      example = ["/var/lib"];
    };
  };

  config = lib.mkIf cfg.enable {
    services.borgbackup.jobs.${cfg.name} = {
      paths = cfg.backupPaths;
      repo = cfg.borgRepo;
      encryption = {
        mode = "repokey-blake2";
        passCommand = "cat ${config.age.secrets.mediaBorgPass.path}";
      };
      compression = "auto,lzma";
      startAt = cfg.backupFrequency;
      prune.keep = {
        within = "1d";
        daily = 7;
        weekly = 4;
        monthly = -1;
      };
    };

    age.secrets.mediaBorgPass.file = ../../secrets/mediaBorgPass.age;
  };
}
