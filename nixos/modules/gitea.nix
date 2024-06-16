{
  lib,
  config,
  ...
}: let
  cfg = config.common.gitea;
in {
  options.common.gitea = {
    enable = lib.mkEnableOption "Enable Gitea";
    backupDir = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Gitea backup path";
      example = "/data/backups/gitea";
    };
    postgresBackupDir = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Gitea Postgres DB backup path";
      example = "/data/backups/postgresql";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "git.pve.elmurphy.com";
      description = "Domain for Gitea";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of Gitea host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 3001;
      description = "Port for Gitea";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    services = {
      gitea = {
        enable = true;
        database = {
          type = "postgres";
          passwordFile = config.age.secrets.giteaDbPass.path;
        };
        dump = {
          enable =
            if cfg.backupDir != null
            then true
            else false;
          backupDir = cfg.backupDir;
        };
        settings.server = {
          DOMAIN = "${cfg.domain}";
          ROOT_URL = "https://${cfg.domain}/";
          HTTP_PORT = cfg.port;
        };
      };
      postgresql = {
        enable = true;
        ensureDatabases = [config.services.gitea.user];
        ensureUsers = [
          {
            name = config.services.gitea.database.user;
            ensureDBOwnership = true;
          }
        ];
      };
      postgresqlBackup = {
        enable =
          if cfg.postgresBackupDir != null
          then true
          else false;
        location = cfg.postgresBackupDir;
        databases = [config.services.gitea.database.name];
        startAt = "*-*-* 23:15:00";
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          };
        };
      };
    };

    age.secrets = {
      giteaDbPass = {
        file = ../../secrets/giteaDbPass.age;
        owner = config.services.gitea.user;
      };
    };
  };
}
