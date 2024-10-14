{
  lib,
  config,
  ...
}: let
  cfg = config.common.gitlab;
in {
  options.common.gitlab = {
    enable = lib.mkEnableOption "Enable gitlab";
    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/gitlab";
      description = "gitlab state path";
    };
    backupDir = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "gitlab backup path";
      example = "/data/backups/gitlab";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "gitlab.pve.elmurphy.com";
      description = "Domain for gitlab";
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
      gitlab = {
        enable = true;
        databasePasswordFile = config.age.secrets.gitlabDbPass.path;
        initialRootPasswordFile = config.age.secrets.gitlabPass.path;
        https = true;
        host = cfg.domain;
        port = 443;
        statePath = cfg.dataDir;
        secrets = {
          dbFile = config.age.secrets.gitlabDbFile.path;
          secretFile = config.age.secrets.gitlabSecretFile.path;
          otpFile = config.age.secrets.gitlabOtpFile.path;
          jwsFile = config.age.secrets.gitlabJwsFile.path;
        };
        backup = {
          path = toString cfg.backupDir;
          startAt = "03:00";
        };
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://unix:/run/gitlab/gitlab-workhorse.socket";
          };
        };
      };
    };

    age.secrets = {
      gitlabPass = {
        file = ../../secrets/gitlabPass.age;
        owner = config.services.gitlab.user;
      };
      gitlabDbPass = {
        file = ../../secrets/gitlabDbPass.age;
        owner = config.services.gitlab.user;
      };
      gitlabDbFile = {
        file = ../../secrets/gitlabDbFile.age;
        owner = config.services.gitlab.user;
      };
      gitlabJwsFile = {
        file = ../../secrets/gitlabJwsFile.age;
        owner = config.services.gitlab.user;
      };
      gitlabOtpFile = {
        file = ../../secrets/gitlabOtpFile.age;
        owner = config.services.gitlab.user;
      };
      gitlabSecretFile = {
        file = ../../secrets/gitlabSecretFile.age;
        owner = config.services.gitlab.user;
      };
    };
  };
}
