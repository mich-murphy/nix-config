{
  lib,
  config,
  ...
}: let
  cfg = config.common.rss-bridge;
in {
  options.common.rss-bridge = {
    enable = lib.mkEnableOption "Enable rss-bridge";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "rss-bridge.pve.elmurphy.com";
      description = "Domain for rss-bridge";
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
      rss-bridge = {
        enable = true;
        virtualHost = cfg.domain;
        # Reference: https://github.com/RSS-Bridge/rss-bridge/blob/master/config.default.ini.php
        config = {
          system.enabled_bridges = ["*"];
          error = {
            output = "http";
            report_limit = 5;
          };
          FileCache = {
            enable_purge = true;
          };
        };
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
        };
      };
    };
  };
}
