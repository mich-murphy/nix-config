{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.sabnzbd;
in {
  options.common.sabnzbd = {
    enable = mkEnableOption "Enable Sabdnzbd";
    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/sabnzbd";
      description = "Path to Sabnzbd config";
    };
    completeDir = mkOption {
      type = types.str;
      description = "Path to Sabnzbd complete downloads";
      example = "/mnt/data/downloads/nzb/complete";
    };
    incompleteDir = mkOption {
      type = types.str;
      description = "Path to Sabnzbd incomplete downloads";
      example = "/mnt/data/downloads/nzb/incomplete";
    };
    domain = mkOption {
      type = types.str;
      default = "sabnzbd.pve.elmurphy.com";
      description = "Domain for Sabnzbd";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP for Sabnzbd host";
    };
    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port for Sabnzbd";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
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
      };
    };

    services.nginx = mkIf cfg.nginx {
      virtualHosts.${cfg.domain} = {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          proxyWebsockets = true;
        };
      };
    };
  };
}
