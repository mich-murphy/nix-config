{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.nextcloud;
in
{
  options.common.nextcloud = {
    enable = mkEnableOption "Enable Nextcloud with Postgres DB, Redis caching and automatic DNS validation";
  };

  config = mkIf cfg.enable {
    services = {
      nextcloud = {
        enable = true;
        package = pkgs.nextcloud26;
        hostName = "nextcloud.pve.elmurphy.com";
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
