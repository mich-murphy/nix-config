{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    tailscale.enable = true;
    acme.enable = true;
    nextcloud = {
      enable = true;
      dataDir = "/data/nextcloud";
      postgresqlBackupDir = "/data/backups/postgresql";
      borgbackup = {
        enable = true;
        repo = "ssh://duqvv98y@duqvv98y.repo.borgbase.com/./repo";
      };
    };
    borgbackup = {
      enable = true;
      borgRepo = "ssh://g268tdfo@g268tdfo.repo.borgbase.com/./repo";
      backupPaths = [
        "/var/lib/audiobookshelf"
        "/var/lib/freshrss"
        "/var/lib/forgejo"
        "/var/lib/jellyfin"
        "/var/lib/komga"
        "/var/lib/lidarr"
        "/var/lib/prowlarr"
        "/var/lib/radarr"
        "/var/lib/readarr"
        "/var/lib/sonarr"
        "/var/lib/sabnzbd"
        "/var/lib/pinchflat"
      ];
    };
    komga = {
      enable = true;
      extraGroups = ["media"];
    };
    freshrss.enable = true;
    plex = {
      enable = true;
      extraGroups = ["media"];
      enableAudnexus = true;
      enableTautulli = true;
    };
    audiobookshelf = {
      enable = true;
      extraGroups = ["media"];
    };
    forgejo = {
      enable = true;
      backupDir = "/data/backups/forgejo";
    };
    pinchflat = {
      enable = true;
      mediaDir = "/mnt/data/media/youtube";
    };
    sabnzbd = {
      enable = true;
      completeDir = "/mnt/data/downloads/nzb/complete";
      incompleteDir = "/mnt/data/downloads/nzb/incomplete";
      port = 8182;
    };
    prowlarr.enable = true;
    radarr = {
      enable = true;
      group = "media";
    };
    sonarr = {
      enable = true;
      group = "media";
    };
    lidarr = {
      enable = true;
      group = "media";
    };
    readarr = {
      enable = true;
      group = "media";
    };
  };

  environment.systemPackages = [
    pkgs.vim
    pkgs.tmux
    pkgs.lazydocker
    pkgs.cifs-utils # used for samba
    # disk usage tooling
    pkgs.du-dust
    pkgs.dua
    pkgs.duf
    # stress testing
    pkgs.s-tui
    pkgs.stress-ng
  ];

  services = {
    qemuGuest.enable = true; # used for hypervisor operations
    nginx = {
      # reverse proxy to other services
      virtualHosts."deluge.pve.elmurphy.com" = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://100.69.115.120:8112";
          proxyWebsockets = true;
        };
      };
    };
  };

  virtualisation = {
    docker = {
      enable = true; # use docker for virtualisation
      autoPrune = {
        enable = true;
        dates = "weekly"; # prune docker resources weekly
      };
    };
  };
}
