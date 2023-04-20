{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.freshrss;
in
{
  options.common.freshrss = {
    enable = mkEnableOption "Enable FreshRSS with Postgres DB";
  };

  config = mkIf cfg.enable {
    services = {
      freshrss = {
        enable = true;
        defaultUser = "mm";
        passwordFile = config.age.secrets.freshrssPass.path;
        baseUrl = "http://10.77.2.9";
        database = {
          type = "pgsql";
          name = "freshrss";
          user = "freshrss";
          host = "/run/postgresql";
        };
      };    
      postgresql = {
        enable = true;
        ensureDatabases = [ "freshrss" ];
        ensureUsers = [{
          name = "freshrss";
          ensurePermissions."DATABASE freshrss" = "ALL PRIVILEGES";
        }];
      };
    };

    systemd = {
      services."freshrss-setup" = {
        requires = [ "postgresql.service" ];
        after = [ "postgresql.service" ];
      };
    };

    age.secrets.freshrssPass.file = ../../secrets/freshrssPass.age;
  };
}
