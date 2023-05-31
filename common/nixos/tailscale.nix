{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.tailscale;
in
{
  options.common.tailscale = {
    enable = mkEnableOption "Enable and authenticate to Tailscale";
    authKeyFile = mkOption {
      type = types.str;
      default = config.age.secrets.tailscaleAuthKey.path;
      description = "Tailscale authentication details";
    };
    enableSSH = mkOption {
      type = types.bool;
      default = true;
      description = "Enable SSH via Tailscale";
    };
    advertiseRoutes = mkOption {
      type = types.str;
      default = "";
      description = "Advertise routes to other machines on network via Tailscale";
      example = "192.168.1.0/24";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.authkeyFile != "";
        message = "authkeyFile must be set";
      }
    ];

    systemd.services.tailscale-autoconnect = {
      description = "Automatic connection to Tailscale";

      # make sure tailscale is running before trying to connect to tailscale
      after = ["network-pre.target" "tailscale.service"];
      wants = ["network-pre.target" "tailscale.service"];
      wantedBy = ["multi-user.target"];

      serviceConfig.Type = "oneshot";

      script = with pkgs; ''
        # wait for tailscaled to settle
        sleep 2

        # check if we are already authenticated to tailscale
        status="$(${tailscale}/bin/tailscale status -json | ${jq}/bin/jq -r .BackendState)"
        # if status is not null, then we are already authenticated
        echo "tailscale status: $status"
        if [ "$status" != "NeedsLogin" ]; then
        exit 0
        fi

        # otherwise authenticate with tailscale
        # timeout after 10 seconds to avoid hanging the boot process
        ${coreutils}/bin/timeout 10 ${tailscale}/bin/tailscale up \
        --authkey=$(cat "${cfg.authkeyFile}")

        # we have to proceed in two steps because some options are only available
        # after authentication
        ${coreutils}/bin/timeout 10 ${tailscale}/bin/tailscale up \
        ${lib.optionalString (cfg.enableSSH == true) "--ssh"} \
        ${lib.optionalString (cfg.advertiseRoutes != "") "--advertise-routes=${cfg.advertiseRoutes}"} \
      '';
    };

    networking.firewall = {
      trustedInterfaces = [ "tailscale0" ];
      allowedUDPPorts = [ config.services.tailscale.port ];
    };

    services.tailscale = {
      enable = true;
      useRoutingFeatures = if cfg.advertiseRoutes != "" then "server" else "client";
    };

    age.secrets.tailscaleAuthKey.file = ../../secrets/tailscaleAuthKey.age;
  };
 }
