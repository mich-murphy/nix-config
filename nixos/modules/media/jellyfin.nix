{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.jellyfin;
in {
  options.common.jellyfin = {
    enable = mkEnableOption "Enable Jellyfin with hardware transcoding";
    extraGroups = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Additional groups for jellyfin user";
      example = ["media"];
    };
    domain = mkOption {
      type = types.str;
      default = "jellyfin.pve.elmurphy.com";
      description = "Domain for Jellyfin";
    };
    hostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address of Jellyfin host";
    };
    nginx = mkOption {
      type = types.bool;
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

  config = mkIf cfg.enable {
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

    services = {
      jellyfin = {
        enable = true;
        openFirewall = true;
      };
      nginx = mkIf cfg.nginx {
        clientMaxBodySize = "20m"; # The default (1M) might not be enough for some posters, etc.
        virtualHosts.${cfg.domain} = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
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
