{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.freshrss;
in
{
  options.common.freshrss = {
    enable = mkEnableOption "Enable FreshRSS";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      freshrss = {
        enable = true;
        defaultUser = "mm";
        passwordFile = config.age.secrets.freshrssPass.path;
        baseUrl = "https://127.0.0.1";
        virtualHost = if cfg.nginx then "freshrss.pve.elmurphy.com" else null;
        database = {
          name = "freshrss";
          user = "freshrss";
          passFile = config.age.secrets.freshrssPass.path;
        };
      };    
      nginx = mkIf cfg.nginx {
        virtualHosts.${config.services.freshrss.virtualHost}= {
          enableACME = true;
          addSSL = true;
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
