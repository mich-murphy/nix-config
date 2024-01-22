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
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      nextcloud = {
        enable = true;
        package = pkgs.nextcloud28;
        extraApps = with config.services.nextcloud.package.packages.apps; {
          inherit contacts calendar notes tasks;
        };
        hostName = "nextcloud.pve.elmurphy.com";
        datadir = "/data/nextcloud";
        database.createLocally = true;
        autoUpdateApps.enable = true;
        https = true;
        caching.redis = true;
        maxUploadSize = "1G";
        config = {
          dbtype = "pgsql";
          dbname = "nextcloud";
          dbuser = "nextcloud";
          adminuser = "admin";
          adminpassFile = config.age.secrets.nextcloudPass.path;
        };
        extraOptions = {
          default_phone_region = "AU";
          redis = {
            host = "127.0.0.1";
            port = 31638;
            dbindex = 0;
            timeout = 1.5;
          };
        };
        phpOptions = {
          output_buffering = "0";
          "opcache.interned_strings_buffer" = "12";
        };
      };
      postgresql = {
        enable = true;
        ensureDatabases = ["nextcloud"];
        ensureUsers = [
          {
            name = "nextcloud";
            ensureDBOwnership = true;
          }
        ];
      };
      postgresqlBackup = {
        enable = true;
        location = "/data/backups/postgresql";
        databases = ["nextcloud"];
        startAt = "*-*-* 23:15:00";
      };
      redis.servers.nextcloud = {
        enable = true;
        port = 31638;
        bind = "127.0.0.1";
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."${config.services.nextcloud.hostName}" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
        };
      };
    };

    systemd = {
      services."nextcloud-setup" = {
        requires = ["postgresql.service"];
        after = ["postgresql.service"];
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
