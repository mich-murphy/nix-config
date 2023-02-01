{ lib, config, pkgs, inputs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./modules/object-storage.nix
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
    groups = {
      object-storage = {};
      duplicati = {};
      syncthing = {};
      nextcloud = {};
    };
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
      object-storage = {
        group = "object-storage";
        isSystemUser = true;
        createHome = true;
        home = "/srv/object-storage";
      };
      duplicati = {
        group = "duplicati";
        isSystemUser = true;
        createHome = true;
        home = "/srv/duplicati";
      };
      syncthing = {
        group = "syncthing";
        isSystemUser = true;
        createHome = true;
        home = "/srv/syncthing";
      };
      nextcloud = {
        group = "nextcloud";
        isSystemUser = true;
        createHome = true;
        home = "/srv/nextcloud";
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
    object-storage.enable = false;
    qemuGuest.enable = true;
    roon-server.enable = true;
    tailscale.enable = true;
    duplicati = {
      enable = true;
      user = "duplicati";
      dataDir = "/srv/duplicati";
      interface = "0.0.0.0";
    };
    syncthing = {
      enable = false;
      user = "syncthing";
      group = "syncthing";
      dataDir = "/srv/syncthing";
      configDir = "/srv/syncthing/.config/syncthing";
      guiAddress = "0.0.0.0:8384";
      overrideDevices = true;
      overrideFolders = true;
      extraOptions = {
        gui = {
          user = "mm";
          password = config.age.secrets.syncthingPass.path;
        };
      };
      devices = {
        "seedbox" = { id = config.age.secrets.syncthingDevice.path; };
      };
      folders = {
        "Music" = {
          path = "/data/media/music";
          devices = [ "seedbox" ];
          type = "receiveonly";
          ignoreDelete = true;
        };
      };
    };
    nextcloud = {
      enable = false;
      package = pkgs.nextcloud25;
      home = "/srv/nextcloud";
      hostName = "nix-media.zonkey-goblin.ts.net";
      autoUpdateApps.enable = true;
      https = true;
      config = {
        overwriteProtocol = "https";
        dbtype = "pgsql";
        dbuser = "nextcloud";
        dbhost = "/run/postgresql";
        dbname = "nextcloud";
        adminuser = "admin";
        adminpassFile = config.age.secrets.nextcloudPass.path;
        defaultPhoneRegion = "AU";
      };
    };    
    postgresql = {
      enable = false;
      ensureDatabases = [ "nextcloud" ];
      ensureUsers = [{
        name = "nextcloud";
        ensurePermissions."DATABASE nextcloud" = "ALL PRIVILEGES";
      }];
    };
    nginx.virtualHosts.${config.services.nextcloud.hostName} = {
      forceSSL = true;
      # generate with `sudo tailscale cert nix-media.zonkey-goblin.ts.net && sudo chmod 644 *.key`
      sslCertificate = "/etc/nixos/secrets/nix-media.zonkey-goblin.ts.net.crt";
      sslTrustedCertificate = "/etc/nixos/secrets/nix-media.zonkey-goblin.ts.net.crt";
      sslCertificateKey = "/etc/nixos/secrets/nix-media.zonkey-goblin.ts.net.key";
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

  systemd = {
    services."nextcloud-setup" = {
      requires = [ "postgresql.service" ];
      after = [ "postgresql.service" ];
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
    nextcloudPass = {
      file = ../../secrets/nextcloudPass.age;
      mode = "770";
      owner = "nextcloud";
      group = "nextcloud";
    };
    syncthingDevice = {
      file = ../../secrets/syncthingDevice.age;
      mode = "770";
      owner = "syncthing";
      group = "syncthing";
    };
    syncthingPass = {
      file = ../../secrets/syncthingPass.age;
      mode = "770";
      owner = "syncthing";
      group = "syncthing";
    };
    objectStorage = {
      file = ../../secrets/objectStorage.age;
      mode = "770";
      owner = "object-storage";
      group = "object-storage";
    };
  };
}
