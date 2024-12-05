{pkgs, ...}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    stirling-pdf = {
      enable = true;
      nginx = false;
      hostAddress = "0.0.0.0";
    };
    tailscale.enable = true;
    # deluge = {
    #   enable = true;
    #   nginx = false;
    #   downloadDir = "/mnt/torrents";
    #   torrentDir = "/srv/torrents/watch";
    # };
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
