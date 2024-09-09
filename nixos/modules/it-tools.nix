{
  lib,
  config,
  ...
}: let
  cfg = config.common.ittools;
in {
  options.common.ittools = {
    enable = lib.mkEnableOption "Enable IT-Tools";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "it-tools.pve.elmurphy.com";
      description = "Domain for IT-Tools";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for IT-Tools host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8900;
      description = "Port for IT-Tools";
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
      containers."ittools" = {
        autoStart = true;
        image = "corentinth/it-tools:latest";
        ports = ["${toString cfg.port}:80"];
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
