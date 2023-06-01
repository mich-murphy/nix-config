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
          hostname = "media";
          user = "mm";
          identityFile = "~/.ssh/nix-media";
          setEnv = {
            "LC_ALL" = "C";
          };
        };
        "proxmox" = {
          hostname = "proxmox";
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
