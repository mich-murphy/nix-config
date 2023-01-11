{ config, pkgs, ... }:

let
  impermanence = builtins.fetchTarball {
    url = "https://github.com/nix-community/impermanence/archive/master.tar.gz";
  };
in
{
  imports = [
    ./hardware-configuration.nix
    "${impermanence}/nixos.nix"
    ./modules/s3fs.nix
  ];

  boot.loader.grub.enable = true;
  boot.loader.grub.version = 2;
  boot.loader.grub.device = "/dev/sda";

  time.timeZone = "Australia/Melbourne";

  i18n.defaultLocale = "en_AU.UTF-8";
  services.xserver.layout = "us";

  users.mutableUsers = false;
  users.users.mm = {
    isNormalUser = true;
    home = "/home/mm";
    passwordFile = "/nix/persist/users/mm";
    extraGroups = [ "wheel" ];
  };

  environment = {
    persistence."/nix/persist" = {
      directories = [
        "/etc/nixos"
        "/srv"
        "/var/log"
        "/var/lib"
        "/data"
      ];
      files = [
        "/etc/machine-id"
        "/users/mm"
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
      rsync
    ];
  };

  services = {
    qemuGuest.enable = true;
    roon-server.enable = true;
    roon-server.openFirewall = true;
    tailscale.enable = true;
    s3fs.enable = true;
    #nextcloud = {
      #enable = false;
      #hostName = "localhost";
      #autoUpdateApps.enable = true;
      #https = true;
      #config = {
        #adminpassFile = "/etc/passwd-nextcloud";
        #extraTrustedDomains = [ "nix-media.zonkey-goblin.ts.net" ];
        #dbtype = "pgsql";
        #defaultPhoneRegion = "AU";
      #};
    #};
    openssh = {
      enable = true;
      allowSFTP = false;
      kbdInteractiveAuthentication = false;
      permitRootLogin = "no";
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
    };
  };

  # Example service config
  #systemd.services.roon-server = {
    #wantedBy = [ "mulit-user.target" ];
    #serviceConfig = {
      #User = "roon-server";
      #Group = "roon-server";
      #ProtectSystem = "full";
      #ProtectHome = true;
      #NoNewPrivileges = true;
    #};
  #};

  # Example service user config
  #users.users.roon-server = {
    #group = "roon-server";
    #isSystemUser = true;
    #createHome = true;
    #home = "/var/lib/roon-server";
  #};

  security = {
    sudo.execWheelOnly = true;
    auditd.enable = true;
    audit.enable = true;
    audit.rules = [
      "-a exit,always -F arch=b64 -S execve"
    ];
  };

  nix = {
    settings.auto-optimise-store = true;
    settings.allowed-users = [ "@wheel" ];
    package = pkgs.nix;
    gc.automatic = true;
    gc.options = "--delete-older-than 7d";
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  nixpkgs.config.allowUnfree = true;
  system.stateVersion = "22.11";

}
