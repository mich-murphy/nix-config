{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.gitea;
in {
  options.common.gitea = {
    enable = mkEnableOption "Enable Gitea";
    backupDir = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Gitea backup path";
      example = "/data/backups/gitea";
    };
    postgresBackupDir = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Gitea Postgres DB backup path";
      example = "/data/backups/postgresql";
    };
    domain = mkOption {
      type = types.str;
      default = "git.pve.elmurphy.com";
      description = "Domain for Gitea";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address of Gitea host";
    };
    port = mkOption {
      type = types.port;
      default = 3001;
      description = "Port for Gitea";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
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
      nginx = mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
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
