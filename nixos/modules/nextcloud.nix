{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.nextcloud;
in {
  options.common.nextcloud = {
    enable = mkEnableOption "Enable Nextcloud with Postgres DB and Redis caching";
    dataDir = mkOption {
      type = types.str;
      description = "Directory for storing Nextcloud data";
      example = "/data/nextcloud";
    };
    postgresqlBackupDir = mkOption {
      type = types.str;
      description = "Directory for storing Nextcloud DB dump";
      example = "/data/backups/postgresql";
    };
    domain = mkOption {
      type = types.str;
      default = "nextcloud.pve.elmurphy.com";
      description = "Domain for Nextcloud service";
    };
    borgbackup = {
      enable = mkEnableOption "Enable borgbackup for Nextcloud";
      repo = mkOption {
        type = types.str;
        description = "Borgbackup repository";
        example = "ssh://duqvv98y@duqvv98y.repo.borgbase.com/./repo";
      };
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    services = {
      nextcloud = {
        enable = true;
        hostName = cfg.domain;
        package = pkgs.nextcloud28;
        datadir = cfg.dataDir;
        database.createLocally = true;
        configureRedis = true;
        https = true;
        maxUploadSize = "1G";
        # appstoreEnable = true;
        autoUpdateApps.enable = true;
        extraApps = with config.services.nextcloud.package.packages.apps; {
          inherit contacts calendar notes tasks;
        };
        settings = {
          default_phone_region = "AU";
          maintenance_window_start = 2;
        };
        config = {
          dbtype = "pgsql";
          dbname = "nextcloud";
          dbuser = "nextcloud";
          adminuser = "admin";
          adminpassFile = config.age.secrets.nextcloudPass.path;
        };
        phpOptions = {
          output_buffering = "0";
          "opcache.interned_strings_buffer" = "12";
        };
      };
      postgresqlBackup = {
        enable = true;
        location = cfg.postgresqlBackupDir;
        databases = ["nextcloud"];
        startAt = "*-*-* 23:15:00";
      };
      borgbackup.jobs = mkIf cfg.borgbackup.enable {
        "nextcloud" = {
          paths = [
            config.services.postgresqlBackup.location
            config.services.nextcloud.datadir
          ];
          repo = cfg.borgbackup.repo;
          encryption = {
            mode = "repokey-blake2";
            passCommand = "cat ${config.age.secrets.nextcloudBorgPass.path}";
          };
          compression = "auto,lzma";
          startAt = "daily";
          prune.keep = {
            within = "1d";
            daily = 7;
            weekly = 4;
            monthly = -1;
          };
        };
      };
      nginx = mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
        };
      };
    };

    age.secrets = {
      nextcloudBorgPass.file = ../../secrets/nextcloudBorgPass.age;
      nextcloudPass = {
        file = ../../secrets/nextcloudPass.age;
        owner = "nextcloud";
      };
    };
  };
}
