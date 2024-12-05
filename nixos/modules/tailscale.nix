{
  lib,
  config,
  ...
}: let
  cfg = config.common.tailscale;
in {
  options.common.tailscale = {
    enable = lib.mkEnableOption "Enable Tailscale";
    authKeyTag = lib.mkOption {
      type = lib.types.str;
      default = "server";
      description = "Tag associated with preauthorized key";
    };
    limitNetworkInterfaces = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Only accept traffix via the Tailscale interface";
    };
  };

  # pre-configure lxc containers in proxmox
  # vi /etc/pve/lxc/<lxc-num>.conf
  # lxc.cgroup.devices.allow: c 10:200 rwm
  # lxc.mount.entry: /dev/net dev/net none bind,create=dir
  config = lib.mkIf cfg.enable {
    services.tailscale = {
      enable = true;
      useRoutingFeatures = "client";
      extraUpFlags = [
        "--ssh"
        "--advertise-tags=tag:${cfg.authKeyTag}"
      ];
      authKeyFile = config.age.secrets.tailscaleAuthKey.path;
      authKeyParameters = {
        ephemeral = false;
      };
    };

    networking.firewall = lib.mkIf cfg.limitNetworkInterfaces {
      trustedInterfaces = ["tailscale0"];
      allowedUDPPorts = [config.services.tailscale.port];
    };

    age.secrets.tailscaleAuthKey.file = ../../secrets/tailscaleAuthKey.age;
  };
}
