{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    tailscale.enable = true;
    acme.enable = true;
    nextcloud.enable = true;
    borgbackup.enable = true;
    komga.enable = true;
    freshrss.enable = true;
    plex.enable = true;
    audiobookshelf.enable = true;
    gitea.enable = true;
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
