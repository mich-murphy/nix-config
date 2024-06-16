{
  lib,
  config,
  ...
}:
let
  cfg = config.common.pinchflat;
in {
  options.common.pinchflat = {
    enable = lib.mkEnableOption "Enable Pinchflat";
    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/pinchflat";
      description = "Path to Pinchflat config";
    };
    mediaDir = lib.mkOption {
      type = lib.types.str;
      description = "Path to media";
      example = "/mnt/data/media/youtube";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "pinchflat.pve.elmurphy.com";
      description = "Domain for Pinchflat";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for Pinchflat host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8945;
      description = "Port for Pinchflat";
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
      {
        assertion = cfg.mediaDir != null;
        message = "Specify a path for media";
      }
    ];

    virtualisation.oci-containers = {
      backend = "docker";
      containers."pinchflat" = {
        autoStart = true;
        image = "ghcr.io/kieraneglin/pinchflat:latest";
        ports = ["${toString cfg.port}:8945"];
        volumes = [
          "${cfg.dataDir}:/config"
          "${cfg.mediaDir}:/downloads"
        ];
      };
    };
    services.nginx = lib.mkIf cfg.nginx {
      virtualHosts.${cfg.domain} = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          proxyWebsockets = true;
          extraConfig = ''
            proxy_set_header X-Forwarded-Ssl on;
          '';
        };
      };
    };
  };
}
