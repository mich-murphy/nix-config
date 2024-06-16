{
  lib,
  config,
  ...
}: let
  cfg = config.common.tailscale;
in {
  options.common.tailscale = {
    enable = lib.mkEnableOption "Enable Tailscale";
    limitNetworkInterfaces = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Only accept traffix via the Tailscale interface";
    };
  };

  config = lib.mkIf cfg.enable {
    services.tailscale = {
      enable = true;
      useRoutingFeatures = "client";
    };

    networking.firewall = {
      trustedInterfaces =
        if cfg.limitNetworkInterfaces
        then ["tailscale0"]
        else [];
      allowedUDPPorts = [config.services.tailscale.port];
    };
  };
}
