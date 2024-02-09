{
  config,
  pkgs,
  inputs,
  ...
}: {
  imports = [
    ../../common/nixos
    ./hardware-configuration.nix
    inputs.impermanence.nixosModules.impermanence
  ];

  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";
  };

  networking.hostName = "media";
  time.timeZone = "Australia/Melbourne";

  users = {
    groups.media = {};
    groups.media.gid = 985;
    mutableUsers = false;
    users = {
      mm = {
        isNormalUser = true;
        home = "/home/mm";
        hashedPasswordFile = config.age.secrets.userPass.path;
        extraGroups = ["wheel" "media"];
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEVyN0R5mTtfcbkmVXjicuvSRotJY4IOuetOgPyG2lg8"
        ];
      };
    };
  };

  environment = {
    persistence."/nix/persist" = {
      directories = [
        "/etc/nixos"
        "/var/log"
        "/var/lib"
        "/root/.ssh"
      ];
    };
    etc = {
      "machine-id".source = "/nix/persist/etc/machine-id";
      "ssh/ssh_host_rsa_key".source = "/nix/persist/etc/ssh/ssh_host_rsa_key";
      "ssh/ssh_host_rsa_key.pub".source = "/nix/persist/etc/ssh/ssh_host_rsa_key.pub";
      "ssh/ssh_host_ed25519_key".source = "/nix/persist/etc/ssh/ssh_host_ed25519_key";
      "ssh/ssh_host_ed25519_key.pub".source = "/nix/persist/etc/ssh/ssh_host_ed25519_key.pub";
    };
    systemPackages = [
      pkgs.vim
      pkgs.tmux
      pkgs.docker-compose
      pkgs.lazydocker
      pkgs.cifs-utils
      # disk usage tooling
      pkgs.du-dust
      pkgs.duf
    ];
  };

  fileSystems."/mnt/data" = {
    device = "//10.77.2.102/data";
    fsType = "cifs";
    options = let
      # this line prevents hanging on network split
      automount_opts = "x-systemd.automount,noauto,x-systemd.idle-timeout=60,x-systemd.device-timeout=5s,x-systemd.mount-timeout=5s,dir_mode=0775,file_mode=0775";
    in ["${automount_opts},credentials=${config.age.secrets.sambaPass.path},gid=${toString config.users.groups.media.gid}"];
  };

  common = {
    tailscale.enable = true;
    acme.enable = true;
    nextcloud.enable = true;
    borgbackup.enable = true;
    komga.enable = true;
    freshrss.enable = true;
    plex.enable = true;
    wallabag.enable = true;
    gitea.enable = true;
    ytdlp.enable = true;
    arrs = {
      enable = true;
      enableKapowarr = false;
    };
  };

  services = {
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
  };

  virtualisation = {
    docker = {
      enable = true;
      autoPrune = {
        enable = true;
        dates = "weekly";
      };
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
      builders-use-substitutes = true;
      substituters = [
        "https://nix-community.cachix.org"
      ];
      trusted-public-keys = [
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      ];
      warn-dirty = false;
    };
    package = pkgs.nixUnstable;
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  nixpkgs.config.allowUnfree = true;
  nixpkgs.hostPlatform = "x86_64-linux";
  system.stateVersion = "23.05";

  age.secrets = {
    userPass.file = ../../secrets/userPass.age;
    sambaPass.file = ../../secrets/sambaPass.age;
  };
}
