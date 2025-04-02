{
  lib,
  config,
  ...
}: let
  cfg = config.common.paperless;
in {
  imports = [
    ./borgbackup.nix
  ];

  options.common.paperless = {
    enable = lib.mkEnableOption "Enable paperless";
    extraGroups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Additional groups for plex user";
      example = ["media"];
    };
    mediaDir = lib.mkOption {
      type = lib.types.str;
      description = "Path to media";
      example = "/mnt/data/documents";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "paperless.pve.elmurphy.com";
      description = "Domain for paperless";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address for IT-Tools host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 28981;
      description = "Port for Pinchflat";
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
      {
        assertion = cfg.mediaDir != null;
        message = "Specify a path for media";
      }
    ];

    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [config.services.paperless.dataDir];

    services = {
      paperless = {
        enable = true;
        mediaDir = cfg.mediaDir;
        port = cfg.port;
        passwordFile = config.age.secrets.paperlessPass.path;
        address = cfg.hostAddress;
        settings = {
          PAPERLESS_ADMIN_USER = "mm";
          PAPERLESS_CONSUMER_IGNORE_PATTERN = [
            ".DS_STORE/*"
            "desktop.ini"
          ];
          PAPERLESS_OCR_USER_ARGS = {
            optimize = 1;
            pdfa_image_compression = "lossless";
          };
        };
      };
      nginx = lib.mkIf cfg.nginx {
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

    users.users.paperless.extraGroups = cfg.extraGroups;

    age.secrets = {
      paperlessPass = {
        file = ../../secrets/paperlessPass.age;
        owner = "paperless";
      };
    };
  };
}
