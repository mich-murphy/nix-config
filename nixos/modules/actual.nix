{
  lib,
  config,
  ...
}: let
  cfg = config.common.actual;
in {
  options.common.actual = {
    enable = lib.mkEnableOption "Enable Actual";
    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/actual";
      description = "Path to Actual config";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "actual.pve.elmurphy.com";
      description = "Domain for Actual";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for Actual host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 5006;
      description = "Port for Actual";
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
      containers."actual" = {
        autoStart = true;
        image = "actualbudget/actual-server:latest";
        ports = ["${toString cfg.port}:5006"];
        volumes = [
          "${cfg.dataDir}:/data"
        ];
        extraOptions = ["--dns=1.1.1.1" "--dns=1.0.0.1"];
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
