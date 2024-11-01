{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.jellyfin;
in {
  imports = [
    ../borgbackup.nix
  ];

  options.common.jellyfin = {
    enable = lib.mkEnableOption "Enable Jellyfin with hardware transcoding";
    extraGroups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Additional groups for jellyfin user";
      example = ["media"];
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "jellyfin.pve.elmurphy.com";
      description = "Domain for Jellyfin";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of Jellyfin host";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  # intel transcoding specification
  # https://www.intel.com/content/www/us/en/docs/onevpl/developer-reference-media-intel-hardware/1-0/overview.html

  # jellyfin documentation
  # https://jellyfin.org/docs/general/administration/hardware-acceleration/intel

  # arch wiki documentation
  # https://wiki.archlinux.org/title/Hardware_video_acceleration#Verification

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    hardware.opengl = {
      enable = true;
      extraPackages = [
        pkgs.intel-media-driver
        pkgs.intel-compute-runtime # opencl filter support (hardware tonemapping and subtitle burn-in)
      ];
    };

    environment = {
      sessionVariables.LIBVA_DRIVER_NAME = "iHD"; # force intel-media-driver
      systemPackages = [
        pkgs.linux-firmware # needed for intel graphics support on skylake or newer
        pkgs.intel-gpu-tools # intel_gpu_top allows monitoring of gpu
        pkgs.libva-utils # vainfo allows verification of va-api info
      ];
    };

    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable [config.services.jellyfin.dataDir];

    services = {
      jellyfin = {
        enable = true;
        logDir = "/var/log/jellyfin";
        openFirewall = true;
      };
      nginx = lib.mkIf cfg.nginx {
        clientMaxBodySize = "20m"; # The default (1M) might not be enough for some posters, etc.
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            # https://jellyfin.org/docs/general/networking/#port-bindings
            proxyPass = "http://${cfg.hostAddress}:8096";
            proxyWebsockets = true;
            extraConfig = ''
              # Disable buffering when the nginx proxy gets very resource heavy upon streaming
              proxy_buffering off;
            '';
          };
        };
      };
    };

    users.users.jellyfin.extraGroups = cfg.extraGroups ++ ["render"]; # allow access to internal gpu
  };
}
