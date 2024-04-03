{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.pinchflat;
in {
  options.common.pinchflat = {
    enable = mkEnableOption "Enable Pinchflat";
    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/pinchflat";
      description = "Path to Pinchflat config";
    };
    mediaDir = mkOption {
      type = types.str;
      description = "Path to media";
      example = "/mnt/data/media/youtube";
    };
    domain = mkOption {
      type = types.str;
      default = "pinchflat.pve.elmurphy.com";
      description = "Domain for Pinchflat";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address for Pinchflat host";
    };
    port = mkOption {
      type = types.port;
      default = 8945;
      description = "Port for Pinchflat";
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
