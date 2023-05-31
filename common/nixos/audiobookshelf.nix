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
    hostName = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = "Address of Host running Audiobookshelf";
    };
    port = mkOption {
      type = types.str;
      default = "8000";
      description = "Port Audiobookshelf is advertised on";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = pkgs.audiobookshelf;

    users = {
      groups.audiobookshelf.members = [ "audiobookshelf" ];
      users.audiobookshelf.isSystemUser = true;
    };

    systemd.services.audiobookshelf = with pkgs; {
      enable = true;
      description = "Self-hosted audiobook server for managing and playing audiobooks";
      serviceConfig = {
        Type = "simple";
        WorkingDirectory = "${cfg.workingDir}";
        ExecStart = "${audiobookshelf}/bin/audiobookshelf --host ${cfg.hostName} --port ${cfg.port}";
        ExecReload = "${util-linux}/bin/kill -HUP $MAINPID";
        Restart = "always";
        User = "audiobookshelf";
        Group = "audiobookshelf";
      };
      wantedBy = [ "multi-user.target" ];
      requires = [ "network.target" ];
    };
  };
}
