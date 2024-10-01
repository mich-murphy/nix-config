{
  lib,
  config,
  ...
}: let
  cfg = config.common.watchtower;
in {
  options.common.watchtower = {
    enable = lib.mkEnableOption "Enable Watchtower";
  };

  config = lib.mkIf cfg.enable {
    virtualisation.oci-containers = {
      backend = "docker";
      containers."watchtower" = {
        autoStart = true;
        image = "containrrr/watchtower";
        environment = {
          WATCHTOWER_CLEANUP = "true"; # clean outdated images after update
        };
        volumes = [
          "/var/run/docker.sock:/var/run/docker.sock"
        ];
        extraOptions = ["--dns=1.1.1.1" "--dns=1.0.0.1"];
      };
    };
  };
}
