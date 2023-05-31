{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.tailscale;
in
{
  options.common.tailscale = {
    enable = mkEnableOption "Enable and authenticate to Tailscale";
  };

  config = mkIf cfg.enable {
    services.tailscale = {
      enable = true;
      useRoutingFeatures = "client";
    };

    networking.firewall = {
      trustedInterfaces = [ "tailscale0" ];
      allowedUDPPorts = [ config.services.tailscale.port ];
    };
  };
 }
