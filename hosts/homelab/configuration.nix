{ lib, config, pkgs, inputs, ... }:

let
  audnexusPlugin = pkgs.stdenv.mkDerivation {
    name = "Audnexus.bundle";
    src = pkgs.fetchurl {
      url = https://github.com/djdembeck/Audnexus.bundle/archive/refs/tags/v1.1.0.zip;
      sha256 = "sha256-i5ssEe7SFoQHFXvYiB0nG1mQrcA/wgSeYZiyYKDYtuQ=";
    };
    buildInputs = [ pkgs.unzip ];
    installPhase = "mkdir -p $out; cp -R * $out/";
  };
in
{
  imports = [
    ./hardware-configuration.nix
    ./modules/nextcloud.nix
    inputs.impermanence.nixosModules.impermanence
  ];

  boot.loader.grub = {
    enable = true;
    version = 2;
    device = "/dev/sda";
  };

  time.timeZone = "Australia/Melbourne";

  i18n = {
    defaultLocale = "en_US.UTF-8";
    extraLocaleSettings = {
      LC_ALL = "en_AU.UTF-8";
      LANG = "en_AU.UTF-8";
    };
  };

  users = {
    mutableUsers = false;
    users = {
      mm = {
        isNormalUser = true;
        home = "/home/mm";
        passwordFile = config.age.secrets.userPass.path;
        extraGroups = [ "wheel" ];
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMne13aa88i97xAUqU33dk2FNz+w8OIMGi8LH4BCRFaN"
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
        "/data"
        "/srv"
      ];
      files = [
        "/etc/machine-id"
      ];
    };
    etc = {
      "ssh/ssh_host_rsa_key".source = "/nix/persist/etc/ssh/ssh_host_rsa_key";
      "ssh/ssh_host_rsa_key.pub".source = "/nix/persist/etc/ssh/ssh_host_rsa_key.pub";
      "ssh/ssh_host_ed25519_key".source = "/nix/persist/etc/ssh/ssh_host_ed25519_key";
      "ssh/ssh_host_ed25519_key.pub".source = "/nix/persist/etc/ssh/ssh_host_ed25519_key.pub";
    };
    systemPackages = with pkgs; [
      vim
      tmux
    ];
  };

  services = {
    xserver.layout = "us";
    # object-storage = {
    #   enable = true;
    #   keyPath = config.age.secrets.objectStorage.path;
    # };
    qemuGuest.enable = true;
    roon-server.enable = true;
    tailscale.enable = true;
    plex = {
      enable = true;
      extraPlugins = [ audnexusPlugin ];
    };
    duplicati = {
      enable = true;
      user = "duplicati";
      interface = "0.0.0.0";
    };
    syncthing = {
      enable = true;
      user = "syncthing";
      group = "syncthing";
      dataDir = "/srv/syncthing";
      configDir = "/srv/syncthing/.config/syncthing";
      guiAddress = "0.0.0.0:8384";
      overrideDevices = true;
      overrideFolders = true;
      devices = {
        "seedbox" = {
          id = "5N3E33W-SCXYEL5-URIJLAW-Y32VCKK-UYNSVR2-R5I6KMJ-YZ4CIKB-6SDUOAT";
        };
      };
      folders = {
        "Music" = {
          id = "mrpfh-btugj";
          path = "/data/media/music";
          devices = [ "seedbox" ];
          type = "receiveonly";
        };
        "Audiobooks" = {
          id = "mqh32-k7ykn";
          path = "/data/media/audiobooks";
          devices = [ "seedbox" ];
          type = "receiveonly";
        };
      };
    };
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
    };
  };

  networking = {
    hostName = "nix-media";
    firewall = {
      enable = true;
      trustedInterfaces = [ "tailscale0" ];
      checkReversePath = "loose";
      allowedUDPPorts = [ config.services.tailscale.port ];
      extraCommands = ''
        iptables -A nixos-fw -p tcp --source 10.77.1.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p udp --source 10.77.1.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p tcp --source 10.77.2.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p udp --source 10.77.2.0/24 -j nixos-fw-accept
      '';
    };
  };

  security = {
    sudo.execWheelOnly = true;
    sudo.wheelNeedsPassword = false;
    auditd.enable = true;
    audit.enable = true;
    audit.rules = [
      "-a exit,always -F arch=b64 -S execve"
    ];
  };

  nix = {
    settings.auto-optimise-store = true;
    settings.allowed-users = [ "@wheel" ];
    package = pkgs.nixUnstable;
    gc.automatic = true;
    gc.options = "--delete-older-than 7d";
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  nixpkgs.config.allowUnfree = true;
  system.stateVersion = "22.11";

  age.secrets = {
    userPass.file = ../../secrets/userPass.age;
    # objectStorage.file = ../../secrets/objectStorage.age;
  };
}
