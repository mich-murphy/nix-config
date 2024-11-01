{
  lib,
  config,
  ...
}: let
  cfg = config.common.forgejo;
in {
  options.common.forgejo = {
    enable = lib.mkEnableOption "Enable forgejo";
    backupDir = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "forgejo backup path";
      example = "/data/backups/forgejo";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "git.pve.elmurphy.com";
      description = "Domain for forgejo";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of forgejo host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 3201;
      description = "Port for forgejo";
    };
    borgbackup = {
      enable = lib.mkEnableOption "Enable borgbackup for forgejo";
      repo = lib.mkOption {
        type = lib.types.str;
        description = "Borgbackup repository";
        example = "ssh://duqvv98y@duqvv98y.repo.borgbase.com/./repo";
      };
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
      forgejo = {
        enable = true;
        database.type = "postgres";
        dump = {
          enable =
            if cfg.backupDir != null
            then true
            else false;
          backupDir = cfg.backupDir;
        };
        settings = {
          server = {
            DOMAIN = "${cfg.domain}";
            ROOT_URL = "https://${cfg.domain}/";
            HTTP_PORT = cfg.port;
          };
          # You can temporarily allow registration to create an admin user.
          service.DISABLE_REGISTRATION = true;
        };
      };
      borgbackup.jobs = lib.mkIf cfg.borgbackup.enable {
        "git" = {
          paths = [
            config.services.forgejo.dump.backupDir
            config.services.forgejo.stateDir
          ];
          repo = cfg.borgbackup.repo;
          encryption = {
            mode = "repokey-blake2";
            passCommand = "cat ${config.age.secrets.gitBorgPass.path}";
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
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          extraConfig = ''
            client_max_body_size 512M;
          '';
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          };
        };
      };
    };

    age.secrets.gitBorgPass.file = ../../secrets/gitBorgPass.age;
  };
}
