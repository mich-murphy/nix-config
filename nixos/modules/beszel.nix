{
  lib,
  config,
  ...
}: let
  cfg = config.common.beszel;
in {
  options.common.beszel = {
    enable = lib.mkEnableOption "Enable Beszel";
    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/beszel";
      description = "Path to Beszel config";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "beszel.pve.elmurphy.com";
      description = "Domain for Beszel";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for Beszel host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8090;
      description = "Port for Beszel";
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

    virtualisation.oci-containers = {
      backend = "docker";
      containers."beszel" = {
        autoStart = true;
        image = "henrygd/beszel:latest";
        ports = ["${toString cfg.port}:8090"];
        volumes = [
          "${cfg.dataDir}:/beszel_data"
        ];
        # allow access to clients on vpn
        extraOptions = ["--network=host"];
      };
    };
    services.nginx = lib.mkIf cfg.nginx {
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
}
