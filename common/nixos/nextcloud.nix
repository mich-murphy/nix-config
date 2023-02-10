{ lib, config, pkgs, inputs, ... }:

with lib;

let
  cfg = config.common.nextcloud;
in
{
  options.common.nextcloud = {
    enable = mkEnableOption "Enable Nextcloud with Postgres DB and Redis caching";
  };

  config = mkIf cfg.enable {
    services = {
      nextcloud = {
        enable = true;
        hostName = "nix-media.zonkey-goblin.ts.net";
        autoUpdateApps.enable = true;
        https = true;
        caching.redis = true;
        config = {
          dbtype = "pgsql";
          dbname = "nextcloud";
          dbuser = "nextcloud";
          dbhost = "/run/postgresql";
          adminuser = "admin";
          adminpassFile = config.age.secrets.nextcloudPass.path;
          defaultPhoneRegion = "AU";
        };
        extraOptions = {
          redis = {
            host = "127.0.0.1";
            port = 31638;
            dbindex = 0;
            timeout = 1.5;
          };
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
        startAt = "*-*-* 23:15:00";
      };
      redis.servers.nextcloud = {
        enable = true;
        port = 31638;
        bind = "127.0.0.1";
      };
      nginx = {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts.${config.services.nextcloud.hostName} = {
          forceSSL = true;
          # generate with `sudo tailscale cert nix-media.zonkey-goblin.ts.net && sudo chmod 644 *.key`
          sslCertificate = "/etc/nixos/nix-media.zonkey-goblin.ts.net.crt";
          sslTrustedCertificate = "/etc/nixos/nix-media.zonkey-goblin.ts.net.crt";
          sslCertificateKey = "/etc/nixos/nix-media.zonkey-goblin.ts.net.key";
          locations."/" = {
            proxyPass = "http://127.0.0.1/8080";
            proxyWebsockets = true;
          };
        };
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
        file = ../../secrets/nextcloudPass.age;
        owner = "nextcloud";
      };
    };
  };
}
