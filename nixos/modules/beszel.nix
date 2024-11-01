{
  lib,
  config,
  ...
}: let
  cfg = config.common.beszel;
in {
  imports = [
    ./borgbackup.nix
  ];

  options.common.beszel = {
    enable = lib.mkEnableOption "Enable Beszel";
    agent = {
      enable = lib.mkEnableOption "Enable Beszel agent on host";
      port = lib.mkOption {
        type = lib.types.port;
        default = 45876;
        description = "Port for Bezsel agent";
      };
      key = lib.mkOption {
        type = lib.types.str;
        default = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHCNAXin8BC5BkM5Ei2D/q8lydKu+qZ6OwKYcENpU8lp";
        description = "SSH key to authenticate to Beszel host";
      };
      monitoredDisk = lib.mkOption {
        type = lib.types.str;
        default = "/dev/sda2";
        description = "Monitored disk for I/O stats in Beszel";
      };
    };
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

    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [cfg.dataDir];

    virtualisation.oci-containers = {
      backend = "docker";
      containers = {
        "beszel" = {
          autoStart = true;
          image = "henrygd/beszel:latest";
          ports = ["${toString cfg.port}:8090"];
          volumes = [
            "${cfg.dataDir}:/beszel_data"
          ];
          # allow access to clients on vpn
          extraOptions = ["--network=host"];
        };
        # system monitoring reporting to central beszel host
        "beszel-agent" = lib.mkIf cfg.agent.enable {
          autoStart = true;
          image = "henrygd/beszel-agent:latest";
          environment = {
            PORT = toString cfg.agent.port;
            KEY = cfg.agent.key;
            FILESYSTEM = cfg.agent.monitoredDisk;
          };
          volumes = [
            "/var/run/docker.sock:/var/run/docker.sock:ro"
          ];
          # allow access to clients on vpn
          extraOptions = ["--network=host"];
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
