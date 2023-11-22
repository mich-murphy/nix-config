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
    port = mkOption {
      type = types.port;
      default = 3001;
      description = "Port for Gitea to be advertised on";
    };
    domain = mkOption {
      type = types.str;
      default = "git.pve.elmurphy.com";
      description = "Domain for Gitea to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      gitea = {
        enable = true;
        database = {
          type = "postgres";
          passwordFile = config.age.secrets.giteaDbPass.path;
        };
        dump = {
          enable = true;
          backupDir = "/data/backups/gitea";
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
            # ensurePermissions."DATABASE ${config.services.gitea.database.name}" = "ALL PRIVILEGES";
            ensureDBOwnership = true;
          }
        ];
      };
      postgresqlBackup = {
        enable = true;
        location = "/data/backups/postgresql";
        databases = [config.services.gitea.database.name];
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
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.port}";
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
