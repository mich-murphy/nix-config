{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.tailscale;
in
{
  options.common.tailscale = {
    enable = mkEnableOption "Enable Tailscale";
  };

  config = mkIf cfg.enable {
    services.tailscale = {
      enable = true;
      useRoutingFeatures = "client";
    };

    networking.firewall = {
      trustedInterfaces = [ "tailscale0" ];
      allowedUDPPorts = [ config.services.tailscale.port ];
      extraCommands = ''
        iptables -A nixos-fw -p tcp --source 10.77.1.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p udp --source 10.77.1.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p tcp --source 10.77.2.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p udp --source 10.77.2.0/24 -j nixos-fw-accept
      '';
    };
  };
 }
