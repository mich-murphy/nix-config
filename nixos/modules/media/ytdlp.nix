{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.ytdlp;
in {
  options.common.ytdlp = {
    enable = mkEnableOption "Enable ytdlp-sub";
    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/ytdlp-sub";
      description = "Path to ytdlp-sub config";
    };
    mediaDir = mkOption {
      type = types.str;
      description = "Path to media";
      example = "/mnt/data/media/youtube";
    };
    domain = mkOption {
      type = types.str;
      default = "ytdlp.pve.elmurphy.com";
      description = "Domain for ytdlp-sub";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address for ytdlp-sub host";
    };
    port = mkOption {
      type = types.port;
      default = 5001;
      description = "Port for ytdlp-sub";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
      {
        assertion = cfg.mediaDir != null;
        message = "Specify a path for media";
      }
    ];

    virtualisation.oci-containers = {
      backend = "docker";
      containers."ytdlp-sub" = {
        autoStart = true;
        image = "ghcr.io/jmbannon/ytdl-sub-gui:latest";
        environment = {
          PUID = "1000";
          PGID = "985";
          TZ = "Australia/Melbourne";
        };
        ports = ["${toString cfg.port}:8443"];
        volumes = [
          "${cfg.dataDir}:/config"
          "${cfg.mediaDir}:/tv_shows"
        ];
      };
    };
    services.nginx = mkIf cfg.nginx {
      virtualHosts.${cfg.domain} = {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          proxyWebsockets = true;
        };
      };
    };
  };
}
