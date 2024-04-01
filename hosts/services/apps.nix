{
  config,
  pkgs,
  ...
}: {
  # system applications

  imports = [
    ../../nixos/modules
  ];

  common = {
    tailscale.enable = true;
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

  services = {
    qemuGuest.enable = true; # used for hypervisor operations
    sabnzbd.enable = true;
    deluge = {
      enable = true;
      web.enable = true; # enable web ui
      declarative = true;
      config = {
        download_location = "/mnt/torrents";
        move_completed = false;
        torrentfiles_location = "/srv/torrents/watch"; # watch for newly added torrents
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
  };

  # agenix managed deluge secrets
  age.secrets.delugePass = {
    file = ../../secrets/delugePass.age;
    owner = "deluge";
  };
}
