{
  lib,
  config,
  ...
}: let
  cfg = config.common.suwayomi-server;
in {
  options.common.suwayomi-server = {
    enable = lib.mkEnableOption "Enable suwayomi-server";
    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/suwayomi-server";
      description = "Path to suwayomi-server config";
    };
    mediaDir = lib.mkOption {
      type = lib.types.str;
      default = "/mnt/data/media/manga";
      description = "Path to manga";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "manga.pve.elmurphy.com";
      description = "Domain for suwayomi-server";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for suwayomi-server host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 3010;
      description = "Port for suwayomi-server";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
    services = {
      suwayomi-server = {
        enable = true;
        dataDir = cfg.dataDir;
        settings.server = {
          ip = cfg.hostAddress;
          port = cfg.port;
          localSourcePath = cfg.mediaDir;
          downloadAsCbz = true;
          extensionRepos = [
            "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
          ];
        };
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
