{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.roon-server;
in {
  options.common.roon-server = {
    enable = mkEnableOption "Enable Roon Server";
  };

  config = mkIf cfg.enable {
    services.roon-server.enable = true;

    networking.firewall = {
      extraCommands = ''
        iptables -A nixos-fw -p tcp --source 10.77.1.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p udp --source 10.77.1.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p tcp --source 10.77.2.0/24 -j nixos-fw-accept
        iptables -A nixos-fw -p udp --source 10.77.2.0/24 -j nixos-fw-accept
      '';
    };

    users.users.roon-server.extraGroups = ["media"];
  };
}
