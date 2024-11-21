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
    immich = {
      enable = true;
      borgbackup = {
        enable = true;
        repo = "ssh://c34r51k4@c34r51k4.repo.borgbase.com/./repo";
      };
    };
    borgbackup = {
      enable = true;
      name = "backup";
      borgRepo = "ssh://hu6gjtw9@hu6gjtw9.repo.borgbase.com/./repo";
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
      borgbackup = {
        enable = true;
        repo = "ssh://t43ihzxk@t43ihzxk.repo.borgbase.com/./repo";
      };
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
    ittools.enable = true;
    watchtower.enable = true;
    searxng.enable = true;
    smokeping.enable = true;
    rss-bridge.enable = true;
    mealie.enable = true;
  };

  environment.systemPackages = [
    pkgs.vim
    pkgs.tmux
    pkgs.lazydocker
    pkgs.wezterm
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
      # open-webui access to ollama
      virtualHosts."ollama.pve.elmurphy.com" = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://100.94.130.71:8080";
          proxyWebsockets = true;
        };
      };
      virtualHosts."fileflows.pve.elmurphy.com" = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://100.94.130.71:19200";
          proxyWebsockets = true;
        };
      };
      virtualHosts."docker.pve.elmurphy.com" = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://100.94.130.71:5001";
          proxyWebsockets = true;
        };
      };
      virtualHosts."invokeai.pve.elmurphy.com" = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://100.94.130.71:3050";
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
