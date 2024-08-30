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

  virtualisation.oci-containers = {
    backend = "docker";
    containers."beszel-agent" = {
      autoStart = true;
      image = "henrygd/beszel-agent:latest";
      environment = {
        PORT = "45876";
        KEY = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHCNAXin8BC5BkM5Ei2D/q8lydKu+qZ6OwKYcENpU8lp";
        FILESYSTEM = "/dev/sda4"; # set to the correct filesystem for disk I/O stats
      };
      volumes = [
        "/var/run/docker.sock:/var/run/docker.sock:ro"
      ];
      # allow access to clients on vpn
      extraOptions = ["--network=host"];
    };
  };

  services.qemuGuest.enable = true; # used for hypervisor operations
}
