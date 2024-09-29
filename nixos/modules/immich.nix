{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.immich;
in {
  options.common.immich = {
    enable = lib.mkEnableOption "Enable Immich";
    borgbackup = {
      enable = lib.mkEnableOption "Enable borgbackup for Immich";
      repo = lib.mkOption {
        type = lib.types.str;
        description = "Borgbackup repository";
        example = "ssh://c34r51k4@c34r51k4.repo.borgbase.com/./repo";
      };
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "immich.pve.elmurphy.com";
      description = "Domain for Immich";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      description = "IP address of Immich host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 3001;
      description = "Port for Immich";
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

    services = {
      immich = {
        enable = true;
        mediaLocation = "/mnt/data/photos";
        host = cfg.hostAddress;
        port = cfg.port;
      };
      borgbackup.jobs = lib.mkIf cfg.borgbackup.enable {
        "photos" = {
          paths = [
            config.services.immich.mediaLocation
          ];
          repo = cfg.borgbackup.repo;
          encryption = {
            mode = "repokey-blake2";
            passCommand = "cat ${config.age.secrets.immichBorgPass.path}";
          };
          compression = "auto,lzma";
          startAt = "daily";
          prune.keep = {
            within = "1d";
            daily = 7;
            weekly = 4;
            monthly = -1;
          };
        };
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          extraConfig = ''
            # allow large file uploads
            client_max_body_size 50000M;

            # Set headers
            proxy_set_header Host              $server_addr;
            proxy_set_header X-Real-IP         $remote_addr;
            proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            # enable websockets: http://nginx.org/en/docs/http/websocket.html
            # proxy_http_version 1.1;
            # proxy_set_header   Upgrade    $http_upgrade;
            # proxy_set_header   Connection "upgrade";
            # proxy_redirect     off;

            # set timeout
            proxy_read_timeout 600s;
            proxy_send_timeout 600s;
            send_timeout       600s;
          '';
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          };
        };
      };
    };

    hardware.graphics = {
      enable = true;
      extraPackages = [pkgs.intel-media-driver];
    };

    environment.sessionVariables = {
      LIBVA_DRIVER_NAME = "iHD";
    };

    users.users.immich.extraGroups = ["video" "render" "media"];

    age.secrets.immichBorgPass.file = ../../secrets/immichBorgPass.age;
  };
}
