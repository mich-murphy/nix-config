{
  config,
  pkgs,
  inputs,
  modulesPath,
  ...
}: {
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
    (modulesPath + "/profiles/qemu-guest.nix")
    ./disk-config.nix
  ];

  boot.loader.grub = {
    devices = ["/dev/sda"];
    efiSupport = true;
    efiInstallAsRemovable = true;
  };

  networking = {
    hostName = "services";
    firewall = {
      trustedInterfaces = ["tailscale0"];
      allowedUDPPorts = [config.services.tailscale.port];
    };
  };
  time.timeZone = "Australia/Melbourne";
  i18n.defaultLocale = "en_US.UTF-8";
  nixpkgs = {
    config.allowUnfree = true;
    hostPlatform = "x86_64-linux";
  };

  users = {
    mutableUsers = false;
    users = {
      mm = {
        isNormalUser = true;
        home = "/home/mm";
        hashedPasswordFile = config.age.secrets.userPass.path;
        extraGroups = ["wheel"];
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8"
        ];
      };
    };
  };

  environment = {
    systemPackages = [
      pkgs.vim
      pkgs.tmux
      pkgs.cifs-utils
      # disk usage tooling
      pkgs.du-dust
      pkgs.dua
      pkgs.duf
    ];
  };

  fileSystems."/mnt/torrents" = {
    device = "//10.77.2.102/data/downloads/torrents";
    fsType = "cifs";
    options = let
      # this line prevents hanging on network split
      automount_opts = "x-systemd.automount,noauto,x-systemd.idle-timeout=60,x-systemd.device-timeout=5s,x-systemd.mount-timeout=5s,dir_mode=0775,file_mode=0775";
    in ["${automount_opts},credentials=${config.age.secrets.sambaPass.path},uid=${toString config.users.users.deluge.uid},gid=${toString config.users.groups.deluge.gid}"];
  };

  services = {
    xserver.xkb.layout = "us";
    qemuGuest.enable = true;
    openssh = {
      enable = true;
      allowSFTP = false;
      settings = {
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
      };
      settings = {
        PermitRootLogin = "no";
      };
      extraConfig = ''
        AllowTcpForwarding yes
        X11Forwarding no
        AllowAgentForwarding no
        AllowStreamLocalForwarding no
        AuthenticationMethods publickey
      '';
      # default to ed25519 key generation
      hostKeys = [
        {
          path = "/etc/ssh/ssh_host_ed25519_key";
          type = "ed25519";
        }
      ];
    };
    deluge = {
      enable = true;
      web.enable = true;
      declarative = true;
      config = {
        download_location = "/mnt/torrents";
        move_completed = false;
        torrentfiles_location = "/srv/torrents/watch";
        random_port = false;
        max_connections_global = 50;
        max_upload_slots_global = -1;
        max_active_seeding = -1;
        max_active_downloading = -1;
        max_active_limit = -1;
        share_ratio_limit = -1;
        seed_time_ratio_limit = -1;
        seed_time_limit = -1;
        enabled_plugins = ["AutoAdd" "Label"];
        listen_ports = [25565 25565];
        outgoing_interface = 25565;
      };
      authFile = config.age.secrets.delugePass.path;
      openFirewall = true;
    };
    tailscale = {
      enable = true;
      useRoutingFeatures = "client";
    };
  };

  security = {
    sudo.execWheelOnly = true;
    sudo.wheelNeedsPassword = false;
  };

  nix = {
    registry.nixpkgs.flake = inputs.nixpkgs;
    gc = {
      automatic = true;
      options = "--delete-older-than 7d";
    };
    settings = {
      auto-optimise-store = true;
      allowed-users = ["@wheel"];
      substituters = [
        "https://nix-community.cachix.org"
      ];
      trusted-public-keys = [
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      ];
    };
    package = pkgs.nixUnstable;
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  system.stateVersion = "23.11";

  age.secrets = {
    userPass.file = ../../secrets/userPass.age;
    sambaPass.file = ../../secrets/sambaPass.age;
    delugePass = {
      file = ../../secrets/delugePass.age;
      owner = "deluge";
    };
  };
}
