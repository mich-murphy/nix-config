{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.jellyfin;
in
{
  options.common.jellyfin = {
    enable = mkEnableOption "Enable Jellyfin with hardware transcoding";
  };

  config = mkIf cfg.enable {
    nixpkgs.config.packageOverrides = pkgs: {
      vaapiIntel = pkgs.vaapiIntel.override { enableHybridCodec = true; };
    };

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
    boot.kernelParams = [ "i915.force_probe=4692" ];

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

    users.users.jellyfin.extraGroups = [ "render" ];

    services.jellyfin.enable = true;
  };
}
