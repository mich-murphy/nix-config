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
    domain = mkOption {
      type = types.str;
      default = "nextcloud.pve.elmurphy.com";
      description = "Hostname for Nextcloud service";
    };
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
        hostName = cfg.domain;
        package = pkgs.nextcloud28;
        datadir = "/data/nextcloud";
        database.createLocally = true;
        configureRedis = true;
        https = true;
        maxUploadSize = "1G";
        # appstoreEnable = true;
        autoUpdateApps.enable = true;
        extraApps = with config.services.nextcloud.package.packages.apps; {
          inherit contacts calendar notes tasks;
        };
        settings.default_phone_region = "AU";
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
        location = "/data/backups/postgresql";
        databases = ["nextcloud"];
        startAt = "*-*-* 23:15:00";
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."${cfg.domain}" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
        };
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
