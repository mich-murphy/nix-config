{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.ssh;
in
{
  options.common.ssh = {
    enable = mkEnableOption "Enable SSH config with configured hosts";
  };

  config = mkIf cfg.enable {
    programs.ssh = {
      enable = true;
      matchBlocks = {
        "media" = {
          hostname = "10.77.2.234";
          user = "mm";
          identityFile = "~/.ssh/nix-media";
        };
        "seedhost" = {
          hostname = "mole.seedhost.eu";
          user = "mm";
          identityFile = "~/.ssh/seedhost";
        };
      };
    };
  };
}
