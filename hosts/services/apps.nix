{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    tailscale.enable = true;
    deluge = {
      enable = true;
      nginx = false;
      downloadDir = "/mnt/torrents";
      torrentDir = "/srv/torrents/watch";
    };
    netdata.enable = true;
  };

  environment = {
    systemPackages = [
      pkgs.vim
      pkgs.tmux
      pkgs.cifs-utils # used for samba
      # disk usage tooling
      pkgs.du-dust
      pkgs.dua
      pkgs.duf
    ];
  };

  services.qemuGuest.enable = true; # used for hypervisor operations
}
