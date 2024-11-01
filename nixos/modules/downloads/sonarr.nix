{
  lib,
  config,
  ...
}: let
  cfg = config.common.sonarr;
in {
  imports = [
    ../borgbackup.nix
  ];

  options.common.sonarr = {
    enable = lib.mkEnableOption "Enable Sonarr";
    group = lib.mkOption {
      type = lib.types.str;
      default = "sonarr";
      description = "Group for sonarr user";
      example = "media";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "sonarr.pve.elmurphy.com";
      description = "Domain for Sonarr";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Sonarr host";
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

    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [config.services.sonarr.dataDir];

    services = {
      sonarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:8989";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
