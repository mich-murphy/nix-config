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
    hub = {
      enable = lib.mkEnableOption "Enable Beszel hub on target machine";
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
      dataDir = lib.mkOption {
        type = lib.types.str;
        default = "/var/lib/beszel";
        description = "Path to Beszel config";
      };
      nginx = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Enable nginx reverse proxy with SSL";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    virtualisation.oci-containers = {
      backend = "docker";
      containers = {
        "beszel-hub" = lib.mkIf cfg.hub.enable {
          autoStart = true;
          image = "henrygd/beszel:latest";
          ports = ["${toString cfg.hub.port}:8090"];
          volumes = [
            "${cfg.hub.dataDir}:/beszel_data"
          ];
          # allow access to clients on vpn
          extraOptions = ["--network=host"];
        };
        # system monitoring reporting to central beszel host
        "beszel-agent" = {
          autoStart = true;
          image = "henrygd/beszel-agent:latest";
          environment = {
            PORT = toString cfg.port;
            KEY = cfg.key;
            FILESYSTEM = cfg.monitoredDisk;
          };
          volumes = [
            "/var/run/docker.sock:/var/run/docker.sock:ro"
          ];
          # allow access to clients on vpn
          extraOptions = ["--network=host"];
        };
      };
    };

    services.nginx = lib.mkIf cfg.hub.nginx {
      virtualHosts.${cfg.hub.domain} = {
        forceSSL = true;
        useACMEHost = "elmurphy.com";
        locations."/" = {
          proxyPass = "http://${cfg.hub.hostAddress}:${toString cfg.hub.port}";
          proxyWebsockets = true;
        };
      };
    };
  };
}
