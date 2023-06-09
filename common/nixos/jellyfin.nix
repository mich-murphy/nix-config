{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.jellyfin;
in
{
  options.common.jellyfin = {
    enable = mkEnableOption "Enable Jellyfin with hardware transcoding";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    hardware.opengl = {
      enable = true;
      extraPackages = with pkgs; [
        intel-media-driver
        vaapiIntel
        vaapiVdpau
        libvdpau-va-gl
        intel-compute-runtime
      ];
    };

    # specify integrated driver Intel Alder Lake - https://nixos.wiki/wiki/Intel_Graphics
    # boot.kernelParams = [ "i915.force_probe=4692" ];

    environment = {
      sessionVariables = {
        LIBVA_DRIVER_NAME = "iHD";
      };
      systemPackages = with pkgs; [
        linux-firmware 
        intel-gpu-tools
        libva-utils
      ];
    };

    users.users.jellyfin.extraGroups = [ "render" "media" ];

    services = {
      jellyfin = {
        enable = true;
        openFirewall = true;
      };
      jellyseerr = {
        enable = true;
        openFirewall = true;
      };
      nginx = mkIf cfg.nginx {
        virtualHosts."jellyfin.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8096";
            proxyWebsockets = true;
          };
        };
        virtualHosts."jellyseerr.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:5055";
            proxyWebsockets = true;
          };
        };
      };
    };
  };
}
