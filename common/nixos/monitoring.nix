{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.monitoring;
in
{
  options.common.monitoring = {
    enable = mkEnableOption "Enable monitoring with Grafana, Loki and Prometheus";
    grafana-port = mkOption {
      type = types.port;
      default = 2342;
      description = "Port for Grafana to be advertised on";
    };
    prometheus-port = mkOption {
      type = types.port;
      default = 9001;
      description = "Port for Prometheus to be advertised on";
    };
    node-port = mkOption {
      type = types.port;
      default = 9002;
      description = "Port for Prometheus node to be advertised on";
    };
  };

  config = mkIf cfg.enable {
    services = {
      grafana = {
        enable = true;
        settings = {
          server = {
            domain = "grafana.pve.elmurphy.com";
            http_port = cfg.grafana-port;
            http_addr = "127.0.0.1";
          };
        };
        provision = {
          enable = true;
          datasources.settings = {
            datasources.prometheus.url = "http://127.0.0.1:${toString cfg.prometheus-port}";
          };
        };
      };
      prometheus = {
        enable = true;
        port = cfg.prometheus-port;
        exporters = {
          node = {
            enable = true;
            enabledCollectors = [ "systemd" ];
            port = cfg.node-port;
          };
        };
        scrapeConfigs = [
          {
            job_name = "monitoring";
            static_configs = [{
              targets = [ "127.0.0.1:${toString cfg.node-port}" ];
            }];
          }
        ];
      };
      nginx.virtualHosts.${config.services.grafana.domain} = {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString cfg.grafana-port}";
          proxyWebsockets = true;
        };
      };
    };
  };
 }
