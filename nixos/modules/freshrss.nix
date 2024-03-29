{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.freshrss;
in {
  options.common.freshrss = {
    enable = mkEnableOption "Enable FreshRSS";
    defaultUser = mkOption {
      type = types.str;
      default = "mm";
      description = "Default user for FreshRSS login";
    };
    hostname = mkOption {
      type = types.str;
      default = "freshrss.pve.elmurphy.com";
      description = "Hostname for FreshRSS";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      freshrss = {
        enable = true;
        defaultUser = cfg.defaultUser;
        passwordFile = config.age.secrets.freshrssPass.path;
        baseUrl = "https://${cfg.hostname}";
        virtualHost =
          if cfg.nginx
          then cfg.hostname
          else null;
        database = {
          name = "freshrss";
          user = "freshrss";
          passFile = config.age.secrets.freshrssPass.path;
        };
      };
      # creation of cert potentially problematic - deactivate nginx option to provision
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts.${cfg.hostname} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
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
