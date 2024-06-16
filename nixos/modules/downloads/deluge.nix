{
  lib,
  config,
  ...
}: let
  cfg = config.common.deluge;
in {
  options.common.deluge = {
    enable = lib.mkEnableOption "Enable Deluge";
    downloadDir = lib.mkOption {
      type = lib.types.str;
      description = "Path to Deluge downloads";
      example = "/mnt/torrents";
    };
    torrentDir = lib.mkOption {
      type = lib.types.str;
      description = "Path to Deluge torrent watch directory";
      example = "/srv/torrents/watch";
    };
    enableSamba = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable Samba mount for Deluge user";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "deluge.pve.elmurphy.com";
      description = "Domain for Deluge";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Deluge host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8112;
      description = "Port for Deluge";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    services = {
      deluge = {
        enable = true;
        web.enable = true; # enable web ui
        declarative = true;
        config = {
          download_location = cfg.downloadDir;
          move_completed = false;
          torrentfiles_location = cfg.torrentDir; # watch for newly added torrents
          random_port = false; # connection and network settings
          max_connections_global = 50;
          max_upload_slots_global = -1;
          max_active_seeding = -1;
          max_active_downloading = -1;
          max_active_limit = -1;
          share_ratio_limit = -1;
          seed_time_ratio_limit = -1;
          seed_time_limit = -1;
          listen_ports = [25565 25565];
          outgoing_interface = 25565;
          enabled_plugins = ["AutoAdd" "Label"]; # activate builtin plugins
        };
        authFile = config.age.secrets.delugePass.path;
        openFirewall = true; # open firewall ports for seeding
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    # mount shared samba drive as deluge user and group
    fileSystems.${cfg.downloadDir} = lib.mkIf cfg.enableSamba {
      device = "//10.77.2.102/data/downloads/torrents";
      fsType = "cifs";
      options = let
        # this line prevents hanging on network split
        automount_opts = "x-systemd.automount,noauto,x-systemd.idle-timeout=60,x-systemd.device-timeout=5s,x-systemd.mount-timeout=5s,dir_mode=0775,file_mode=0775";
      in ["${automount_opts},credentials=${config.age.secrets.sambaPass.path},uid=${toString config.users.users.deluge.uid},gid=${toString config.users.groups.deluge.gid}"];
    };

    # agenix managed deluge secrets
    age.secrets = {
      sambaPass.file =
        if cfg.enableSamba
        then ../../../secrets/sambaPass.age
        else "";
      delugePass = {
        file = ../../../secrets/delugePass.age;
        owner = "deluge";
      };
    };
  };
}
