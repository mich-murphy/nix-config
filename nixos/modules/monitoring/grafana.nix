{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.grafana;
in {
  options.common.grafana = {
    enable = mkEnableOption "Enable monitoring with Grafana, Loki and Prometheus";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "grafana.pve.elmurphy.com";
      description = "Domain for Grafana";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of host";
    };
    grafanaPort = mkOption {
      type = types.port;
      default = 2342;
      description = "Port for Grafana to be advertised on";
    };
    prometheusPort = mkOption {
      type = types.port;
      default = 9001;
      description = "Port for Prometheus to be advertised on";
    };
    prometheusNodePort = mkOption {
      type = types.port;
      default = 9002;
      description = "Port for Prometheus node to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    services = {
      grafana = {
        enable = true;
        settings = {
          server = {
            domain = cfg.domain;
            http_port = cfg.grafanaPort;
            http_addr = cfg.hostAddress;
          };
        };
      };
      prometheus = {
        enable = true;
        port = cfg.prometheusPort;
        exporters = {
          node = {
            enable = true;
            enabledCollectors = ["systemd"];
            port = cfg.prometheusNodePort;
          };
        };
        scrapeConfigs = [
          {
            job_name = "media";
            static_configs = [
              {
                targets = ["${cfg.hostAddress}:${toString cfg.prometheusNodePort}"];
              }
            ];
          }
        ];
      };
      nginx = mkIf cfg.nginx {
        virtualHosts.${config.services.grafana.settings.server.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.grafanaPort}";
            proxyWebsockets = true;
          };
        };
        virtualHosts."prometheus.pve.elmurphy.com" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.prometheusPort}";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
