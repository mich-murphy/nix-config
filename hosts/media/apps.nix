{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    tailscale.enable = true;
    acme.enable = true;
    nextcloud.enable = true;
    borgbackup = {
      enable = true;
      name = "media";
      borgRepo = "ssh://g268tdfo@g268tdfo.repo.borgbase.com/./repo";
      backupPaths = [
        "/var/lib/audiobookshelf"
        "/var/lib/freshrss"
        "/var/lib/gitea"
        "/var/lib/jellyfin"
        "/var/lib/komga"
        "/var/lib/lidarr"
        "/var/lib/nzbget"
        "/var/lib/prowlarr"
        "/var/lib/radarr"
        "/var/lib/readarr"
        "/var/lib/sonarr"
        "/var/lib/ytdlp-sub/ytdl-sub-configs"
      ];
    };
    komga.enable = true;
    freshrss.enable = true;
    plex.enable = true;
    audiobookshelf.enable = true;
    gitea = {
      enable = true;
      backupDir = "/data/backups/gitea";
      postgresBackupDir = "/data/backups/postgresql";
    };
    ytdlp.enable = true;
    minecraft.enable = true;
    murmur.enable = true;
    arrs = {
      enable = true;
      enableKapowarr = false;
    };
  };

  environment.systemPackages = [
    pkgs.vim
    pkgs.tmux
    pkgs.docker-compose
    pkgs.lazydocker
    pkgs.cifs-utils # used for samba
    # disk usage tooling
    pkgs.du-dust
    pkgs.dua
    pkgs.duf
  ];

  services.qemuGuest.enable = true; # used for hypervisor operations

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
