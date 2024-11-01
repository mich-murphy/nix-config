{
  config,
  lib,
  ...
}: let
  cfg = config.common.searxng;
in {
  imports = [
    ./borgbackup.nix
  ];

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
      default = 9050;
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

    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [cfg.dataDir];

    virtualisation.oci-containers = {
      backend = "docker";
      containers = {
        "searxng-redis" = {
          image = "docker.io/valkey/valkey:8-alpine";
          volumes = [
            "searxng_valkey-data2:/data:rw"
          ];
          cmd = ["valkey-server" "--save" "30" "1" "--loglevel" "warning"];
          extraOptions = [
            "--cap-add=DAC_OVERRIDE"
            "--cap-add=SETGID"
            "--cap-add=SETUID"
            "--cap-drop=ALL" # comment out for first run
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
          extraOptions = [
            "--cap-add=CHOWN"
            "--cap-add=SETGID"
            "--cap-add=SETUID"
            "--cap-drop=ALL" # comment out for first run
            "--dns=1.1.1.1"
            "--dns=1.0.0.1"
          ];
        };
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
