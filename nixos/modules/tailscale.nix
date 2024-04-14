{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.tailscale;
in {
  options.common.tailscale = {
    enable = mkEnableOption "Enable Tailscale";
    limitNetworkInterfaces = mkOption {
      type = types.bool;
      default = true;
      description = "Only accept traffix via the Tailscale interface";
    };
  };

  config = mkIf cfg.enable {
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
