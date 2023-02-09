{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.common.object-storage;
in 
{
  options.common.object-storage = {
    enable = mkEnableOption (lib.mdDoc "Mounts s3 object storage using s3fs");
    keyPath = mkOption {
      type = types.str;
      default = config.age.secrets.objectStorage.path;
      description = "Path to file containing 'access-key:secret-key'";
    };
    mountPath = mkOption {
      type = types.str;
      default = "/mnt/storage";
      description = "File system location for the object storage to be mounted to";
    };
    bucket = mkOption {
      type = types.str;
      default = "s3-storage";
      description = "Name of bucket";
    };
    url = mkOption {
      type = types.str;
      default = "https://ap-south-1.linodeobjects.com/";
      description = "URL to access object storage";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.object-storage = {
      description = "Linode object storage s3fs";
      wantedBy = [ "multi-user.target" ];
      startLimitIntervalSec = 5;
      serviceConfig = {
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
        ExecStopPost = "-${pkgs.fuse-common}/bin/fusermount -u ${cfg.mountPath}";
        KillMode = "process";
        Restart = "on-failure";
      };
    };

    age.secrets.objectStorage.file = ../../secrets/objectStorage.age;
  };
}
