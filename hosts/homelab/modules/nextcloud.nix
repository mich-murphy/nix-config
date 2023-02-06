{ lib, config, pkgs, ... }:

{
  services = {
    nextcloud = {
      enable = true;
      hostName = "nix-media.zonkey-goblin.ts.net";
      autoUpdateApps.enable = true;
      https = true;
      config = {
        dbtype = "pgsql";
        dbname = "nextcloud";
        dbuser = "nextcloud";
        dbhost = "/run/postgresql";
        adminuser = "admin";
        adminpassFile = config.age.secrets.nextcloudPass.path;
        defaultPhoneRegion = "AU";
        extraTrustedDomains = [ "0.0.0.0" ];
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
    postgresqlBackup = {
      enable = true;
      location = "/data/backup/nextclouddb";
      databases = [ "nextcloud" ];
    };
    nginx.virtualHosts.${config.services.nextcloud.hostName} = {
      forceSSL = true;
      # generate with `sudo tailscale cert nix-media.zonkey-goblin.ts.net && sudo chmod 644 *.key`
      sslCertificate = "/etc/nixos/nix-media.zonkey-goblin.ts.net.crt";
      sslTrustedCertificate = "/etc/nixos/nix-media.zonkey-goblin.ts.net.crt";
      sslCertificateKey = "/etc/nixos/nix-media.zonkey-goblin.ts.net.key";
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
      owner = "nextcloud";
    };
  };
}
