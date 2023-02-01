{ config, lib, pkgs, inputs, ... }:

with lib;

let
  cfg = config.services.object-storage;
in {
  options.services.object-storage = {
    enable = mkEnableOption "Mounts s3 object storage using s3fs";
    keyPath = mkOption {
      type = types.str;
      default = config.age.secrets.objectStorage.path;
    };
    mountPath = mkOption {
      type = types.str;
      default = "/mnt/storage";
    };
    bucket = mkOption {
      type = types.str;
      default = "s3-storage";
    };
    url = mkOption {
      type = types.str;
      default = "https://ap-south-1.linodeobjects.com/";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.object-storage = {
      description = "Linode object storage s3fs";
      wantedBy = [ "multi-user.target" ];
      startLimitIntervalSec = 5;
      serviceConfig = {
        User = "object-storage";
        Group = "object-storage";
        ProtectSystem = "full";
        ProtectHome = true;
        NoNewPriviliges = true;
        ReadWritePaths  = "${cfg.mountPath}";
        ExecStartPre = [
          "${pkgs.coreutils}/bin/mkdir -m 777 -pv ${cfg.mountPath}"
          #"${pkgs.e2fsprogs}/bin/chattr +i ${cfg.mountPath}" # stop files being written to unmounted dir
        ];
        ExecStart = let
          options = [
            "passwd_file=${cfg.keyPath}"
            "use_path_request_style"
            "allow_other"
            "url=${cfg.url}"
            "umask=0000"
          ];
        in
          "${pkgs.s3fs}/bin/s3fs ${cfg.bucket} ${cfg.mountPath} -f "
            + lib.concatMapStringsSep " " (opt: "-o ${opt}") options;
        ExecStopPost = "-${pkgs.fuse}/bin/fusermount -u ${cfg.mountPath}";
        KillMode = "process";
        Restart = "on-failure";
      };
    };
  };

  age.secrets.objectStorage.file = ../../../secrets/objectStorage.age;
}
