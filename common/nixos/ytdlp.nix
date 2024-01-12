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
    port = mkOption {
      type = types.port;
      default = 5001;
      description = "Port for ytdlp-sub to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
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
          "/var/lib/ytdlp-sub:/config"
          "/mnt/data/media/youtube:/tv_shows"
          # "/mnt/data/media/movies:/movies"
          # "/mnt/data/media/music:/music"
        ];
      };
    };
    services = {
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."ytdlp.pve.elmurphy.com" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
