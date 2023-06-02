{ lib, config, pkgs, inputs, ... }:

{
  imports = [
    ../../common/nixos
    ./hardware-configuration.nix
    inputs.impermanence.nixosModules.impermanence
  ];

  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";
  };

  time.timeZone = "Australia/Melbourne";
  i18n.defaultLocale = "en_US.UTF-8";

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
      ];
      files = [
        # borgbackup default ssh key location - for sudo user
        "/root/.ssh/id_ed25519"
        "/root/.ssh/id_ed25519.pub"
      ];
    };
    etc = {
      "machine-id".source = "/nix/persist/etc/machine-id";
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

  common = {
    audiobookshelf.enable = true;
    tailscale.enable = true;
    syncthing.enable = true;
    jellyfin.enable = true;
    nginx.enable = true;
    nextcloud.enable = true;
    borgbackup.enable = true;
  #   plex.enable = true;
  #   freshrss.enable = true;
  #   calibre-web.enable = true;
  };

  services = {
    xserver.layout = "us";
    qemuGuest.enable = true;
    roon-server.enable = true;
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
      hostKeys = [{
        path = "/etc/ssh/ssh_host_ed25519_key";
        type = "ed25519";
      }];
    };
  };

  networking = {
    hostName = "media";
    firewall = {
      enable = true;
      allowedTCPPorts = [ 55000 ];
      allowedUDPPorts = [ 55000 ];
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
  system.stateVersion = "23.05";

  age.secrets = {
    userPass.file = ../../secrets/userPass.age;
  };
}
