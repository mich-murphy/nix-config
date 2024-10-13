# Auto-generated using compose2nix v0.3.1-pre.
{
  config,
  pkgs,
  lib,
  ...
}: let
  cfg = config.common.searxng;
in {
  options.common.searxng = {
    enable = lib.mkEnableOption "Enable Searxng";
    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/searxng";
      description = "Path to Searxng config";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "searxng.pve.elmurphy.com";
      description = "Domain for Searxng";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for Searxng host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 9002;
      description = "Port for Searxng";
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
      containers = {
        "searxng-redis" = {
          image = "docker.io/valkey/valkey:8-alpine";
          volumes = [
            "searxng_valkey-data2:/data:rw"
          ];
          cmd = ["valkey-server" "--save" "30" "1" "--loglevel" "warning"];
          log-driver = "journald";
          extraOptions = [
            "--cap-add=DAC_OVERRIDE"
            "--cap-add=SETGID"
            "--cap-add=SETUID"
            "--cap-drop=ALL" # comment out for first run
            "--network-alias=redis"
            "--network=searxng_searxng"
          ];
        };
        "searxng" = {
          image = "docker.io/searxng/searxng:latest";
          environment = {
            "SEARXNG_BASE_URL" = "https://${cfg.domain}/";
            "UWSGI_THREADS" = "4";
            "UWSGI_WORKERS" = "4";
          };
          volumes = [
            "${cfg.dataDir}:/etc/searxng:rw"
          ];
          ports = [
            "${toString cfg.port}:8080/tcp"
          ];
          log-driver = "journald";
          extraOptions = [
            "--cap-add=CHOWN"
            "--cap-add=SETGID"
            "--cap-add=SETUID"
            "--cap-drop=ALL" # comment out for first run
            "--network-alias=searxng"
            "--network=searxng_searxng"
          ];
        };
      };
    };
    systemd = {
      services = {
        "docker-searxng-redis" = {
          serviceConfig = {
            Restart = lib.mkOverride 90 "always";
            RestartMaxDelaySec = lib.mkOverride 90 "1m";
            RestartSec = lib.mkOverride 90 "100ms";
            RestartSteps = lib.mkOverride 90 9;
          };
          after = [
            "docker-network-searxng_searxng.service"
            "docker-volume-searxng_valkey-data2.service"
          ];
          requires = [
            "docker-network-searxng_searxng.service"
            "docker-volume-searxng_valkey-data2.service"
          ];
          partOf = [
            "docker-compose-searxng-root.target"
          ];
          wantedBy = [
            "docker-compose-searxng-root.target"
          ];
        };
        "docker-searxng" = {
          serviceConfig = {
            Restart = lib.mkOverride 90 "always";
            RestartMaxDelaySec = lib.mkOverride 90 "1m";
            RestartSec = lib.mkOverride 90 "100ms";
            RestartSteps = lib.mkOverride 90 9;
          };
          after = [
            "docker-network-searxng_searxng.service"
          ];
          requires = [
            "docker-network-searxng_searxng.service"
          ];
          partOf = [
            "docker-compose-searxng-root.target"
          ];
          wantedBy = [
            "docker-compose-searxng-root.target"
          ];
        };
        "docker-network-searxng_searxng" = {
          path = [pkgs.docker];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStop = "docker network rm -f searxng_searxng";
          };
          script = ''
            docker network inspect searxng_searxng || docker network create searxng_searxng
          '';
          partOf = ["docker-compose-searxng-root.target"];
          wantedBy = ["docker-compose-searxng-root.target"];
        };
        "docker-volume-searxng_valkey-data2" = {
          path = [pkgs.docker];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
          };
          script = ''
            docker volume inspect searxng_valkey-data2 || docker volume create searxng_valkey-data2
          '';
          partOf = ["docker-compose-searxng-root.target"];
          wantedBy = ["docker-compose-searxng-root.target"];
        };
      };

      # Root service
      # When started, this will automatically create all resources and start
      # the containers. When stopped, this will teardown all resources.
      targets."docker-compose-searxng-root" = {
        unitConfig = {
          Description = "Root target generated by compose2nix.";
        };
        wantedBy = ["multi-user.target"];
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
