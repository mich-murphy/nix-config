{
  lib,
  config,
  ...
}: let
  cfg = config.common.loki;
in {
  imports = [
    ../borgbackup.nix
  ];

  options.common.loki = {
    enable = lib.mkEnableOption "Enable log capture with Loki and Promtail";
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of host";
    };
    lokiPort = lib.mkOption {
      type = lib.types.port;
      default = 9003;
      description = "Port for Loki to be advertised on";
    };
    promtailPort = lib.mkOption {
      type = lib.types.port;
      default = 9004;
      description = "Port for Promtail to be advertised on";
    };
  };

  config = lib.mkIf cfg.enable {
    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [config.services.loki.dataDir];

    services = {
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
    };
  };
}
