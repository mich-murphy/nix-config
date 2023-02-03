{ lib, config, pkgs, ... }:

{
  services = {
    nextcloud = {
      enable = true;
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
      enable = true;
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
  };

  systemd = {
    services."nextcloud-setup" = {
      requires = [ "postgresql.service" ];
      after = [ "postgresql.service" ];
    };
  };

  age.secrets = {
    nextcloudPass = {
      file = ../../../secrets/nextcloudPass.age;
      mode = "770";
      owner = "nextcloud";
      group = "nextcloud";
    };
  };
}
