{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.linkding;
in
{
  options.common.linkding = {
    enable = mkEnableOption "Enable Linkding";
    workingDir = mkOption {
      type = types.str;
      default = "/var/lib/linkding";
      description = "Path to Linkding config files";
    };
  };

  config = mkIf cfg.enable {
    virtualisation.oci-containers = {
      backend = "docker";
      containers."linkding" = {
        autoStart = true;
        image = "sissbruecker/linkding:latest";
        ports = [ "9090:9090" ];
        volumes = [
          "${cfg.workingDir}/data:/etc/linkding/data"
        ];
      };
    };
  };
}
