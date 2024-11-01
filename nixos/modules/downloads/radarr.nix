{
  lib,
  config,
  ...
}: let
  cfg = config.common.radarr;
in {
  imports = [
    ../borgbackup.nix
  ];

  options.common.radarr = {
    enable = lib.mkEnableOption "Enable Radarr";
    group = lib.mkOption {
      type = lib.types.str;
      default = "radarr";
      description = "Group for radarr user";
      example = "media";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "radarr.pve.elmurphy.com";
      description = "Domain for radarr";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Radarr host";
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

    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [config.services.radarr.dataDir];

    services = {
      radarr = {
        enable = true;
        group = cfg.group;
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:7878";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
