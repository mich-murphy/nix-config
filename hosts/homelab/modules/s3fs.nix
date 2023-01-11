{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.s3fs;
in {
  options.services.s3fs = {
    enable = mkEnableOption "Mounts s3 object storage using s3fs";
    keyPath = mkOption {
      type = types.str;
      default = "/srv/passwd-s3fs";
    };
    mountPath = mkOption {
      type = types.str;
      default = "/mnt/backup";
    };
    bucket = mkOption {
      type = types.str;
      default = "backup";
    };
    url = mkOption {
      type = types.str;
      default = "https://ap-south-1.linodeobjects.com/";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.s3fs = {
      description = "Linode object storage s3fs";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStartPre = [
          "${pkgs.coreutils}/bin/mkdir -pv ${cfg.mountPath}"
          "${pkgs.e2fsprogs}/bin/chattr +i ${cfg.mountPath}" # stop files being written to unmounted dir
        ];
        ExecStart = let
          options = [
            "passwd_file=${cfg.keyPath}"
            "use_path_request_style"
            "allow_other"
            "url=${cfg.url}"
            "umask=0777"
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
}
