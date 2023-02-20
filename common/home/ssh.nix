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
        "nix-media" = {
          hostname = "nix-media";
          user = "mm";
        };
        "alpha" = {
          hostname = "alpha";
          user = "root";
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
