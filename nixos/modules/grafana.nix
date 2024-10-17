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
    lokiPort = mkOption {
      type = types.port;
      default = 9003;
      description = "Port for Loki to be advertised on";
    };
    promtailPort = mkOption {
      type = types.port;
      default = 9004;
      description = "Port for Loki to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
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
      loki = {
        enable = true;
        configuration = {
          auth_enabled = false;
          server.http_listen_port = cfg.lokiPort;
          common = {
            ring = {
              instance_addr = cfg.hostAddress;
              kvstore.store = "inmemory";
            };
            replication_factor = 1;
            path_prefix = "/tmp/loki";
          };
          schema_config = {
            configs = [
              {
                from = "2024-10-01";
                store = "tsdb";
                object_store = "filesystem";
                schema = "v13";
                index = {
                  prefix = "index_";
                  period = "24h";
                };
              }
            ];
          };
          storage_config = {
            tsdb_shipper = {
              active_index_directory = "${config.services.loki.dataDir}/tsdb-index";
              cache_location = "${config.services.loki.dataDir}/tsdb-cache";
            };
            filesystem = {
              directory = "${config.services.loki.dataDir}/chunks";
            };
          };
          limits_config = {
            reject_old_samples = true;
            reject_old_samples_max_age = "168h";
          };
          table_manager = {
            retention_deletes_enabled = false;
            retention_period = "0s";
          };
        };
      };
      promtail = {
        enable = true;
        configuration = {
          server = {
            http_listen_port = cfg.promtailPort;
            grpc_listen_port = 0;
          };
          positions = {
            filename = "/tmp/positions.yaml";
          };
          clients = [
            {
              url = "http://${cfg.hostAddress}:${toString config.services.loki.configuration.server.http_listen_port}/loki/api/v1/push";
            }
          ];
          scrape_configs = [
            {
              job_name = "journal";
              journal = {
                max_age = "12h";
                labels = {
                  job = "systemd-journal";
                  host = "media";
                };
              };
              relabel_configs = [
                {
                  source_labels = ["__journal__systemd_unit"];
                  target_label = "unit";
                }
              ];
            }
          ];
        };
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
