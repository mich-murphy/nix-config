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
    ../../common/nixos
    ./disk-config.nix
  ];

  boot.loader.grub = {
    devices = ["/dev/sda"];
    efiSupport = true;
    efiInstallAsRemovable = true;
  };

  networking.hostName = "storage";
  time.timeZone = "Australia/Melbourne";
  i18n.defaultLocale = "en_US.UTF-8";
  nixpkgs = {
    config.allowUnfree = true;
    hostPlatform = "x86_64-linux";
  };

  users = {
    groups.media = {};
    mutableUsers = false;
    users = {
      mm = {
        isNormalUser = true;
        home = "/home/mm";
        hashedPasswordFile = config.age.secrets.userPass.path;
        extraGroups = ["wheel" "media"];
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMne13aa88i97xAUqU33dk2FNz+w8OIMGi8LH4BCRFaN"
        ];
      };
    };
  };

  environment = {
    systemPackages = with pkgs; [
      neovim
      tmux
      docker-compose
      lazydocker
      # disk usage tooling
      du-dust
      duf
    ];
  };

  common = {
    roon-server.enable = true;
    tailscale.enable = true;
  };

  services = {
    xserver.layout = "us";
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
  };
}
