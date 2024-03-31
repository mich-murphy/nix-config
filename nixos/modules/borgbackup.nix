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
    name = mkOption {
      type = types.str;
      default = config.networking.hostName;
      description = "Name of Borgbackup job for reference by Systemd";
      example = "media";
    };
    borgRepo = mkOption {
      type = types.str;
      description = "SSH URL of Borg repository";
      example = "ssh://g268tdfo@g268tdfo.repo.borgbase.com/./repo";
    };
    backupFrequency = mkOption {
      type = types.enum ["daily" "weekly" "monthly" "yearly"];
      default = "daily";
      description = "Frequency for files and folders to be backed up";
    };
    backupPaths = mkOption {
      type = with types; nullOr (coercedTo str singleton (listOf str));
      default = null;
      description = "List of backup filepaths";
      example = [
        "/var/lib"
      ];
    };
  };

  config = mkIf cfg.enable {
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
