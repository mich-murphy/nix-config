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

    environment = {
      sessionVariables.LIBVA_DRIVER_NAME = "iHD";
      systemPackages = with pkgs; [
        linux-firmware
        intel-gpu-tools
        libva-utils
      ];
    };

    services = {
      jellyfin = {
        enable = true;
        openFirewall = true;
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."jellyfin.pve.elmurphy.com" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8096";
            proxyWebsockets = true;
            extraConfig = ''
              # Disable buffering when the nginx proxy gets very resource heavy upon streaming
              proxy_buffering off;
              # The default (1M) might not be enough for some posters, etc.
              client_max_body_size 20M;
            '';
          };
        };
      };
    };

    users.users.jellyfin.extraGroups = ["render" "media"];
  };
}
