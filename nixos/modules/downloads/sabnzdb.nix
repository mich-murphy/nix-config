{
  lib,
  config,
  ...
}:
let
  cfg = config.common.sabnzbd;
in {
  options.common.sabnzbd = {
    enable = lib.mkEnableOption "Enable Sabdnzbd";
    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/sabnzbd";
      description = "Path to Sabnzbd config";
    };
    completeDir = lib.mkOption {
      type = lib.types.str;
      description = "Path to Sabnzbd complete downloads";
      example = "/mnt/data/downloads/nzb/complete";
    };
    incompleteDir = lib.mkOption {
      type = lib.types.str;
      description = "Path to Sabnzbd incomplete downloads";
      example = "/mnt/data/downloads/nzb/incomplete";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "sabnzbd.pve.elmurphy.com";
      description = "Domain for Sabnzbd";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP for Sabnzbd host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "Port for Sabnzbd";
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

    # initial setup is done via http://<docker host ip>:<mapped http port>
    # user and password must be specified before custom domain works
    # reference: https://sabnzbd.org/wiki/extra/hostname-check.html
    virtualisation.oci-containers = {
      backend = "docker";
      containers."sabnzbd" = {
        autoStart = true;
        image = "lscr.io/linuxserver/sabnzbd:latest";
        environment = {
          PUID = "1000";
          PGID = "985";
          TZ = "Australia/Melbourne";
        };
        ports = ["${toString cfg.port}:8080"];
        volumes = [
          "${cfg.dataDir}:/config"
          "${cfg.completeDir}:/downloads"
          "${cfg.incompleteDir}:/incomplete-downloads"
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
