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
        intel-compute-runtime # OpenCL filter support (hardware tonemapping and subtitle burn-in)
      ];
    };

    users.users.jellyfin.extraGroups = [ "render" ];

    services.jellyfin.enable = true;
  };
}
