{
  lib,
  config,
  ...
}: let
  cfg = config.common.freshrss;
in {
  options.common.freshrss = {
    enable = lib.mkEnableOption "Enable FreshRSS";
    defaultUser = lib.mkOption {
      type = lib.types.str;
      default = "mm";
      description = "Default user for FreshRSS login";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "freshrss.pve.elmurphy.com";
      description = "Domain for FreshRSS";
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
      freshrss = {
        enable = true;
        defaultUser = cfg.defaultUser;
        passwordFile = config.age.secrets.freshrssPass.path;
        baseUrl = "https://${cfg.domain}";
        virtualHost =
          if cfg.nginx
          then cfg.domain
          else null;
        database = {
          name = "freshrss";
          user = "freshrss";
          passFile = config.age.secrets.freshrssPass.path;
        };
      };
      # creation of cert potentially problematic - deactivate nginx option to provision
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
        };
      };
    };

    age.secrets = {
      freshrssPass = {
        file = ../../secrets/freshrssPass.age;
        owner = "freshrss";
      };
    };
  };
}
