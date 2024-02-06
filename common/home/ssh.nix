{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.ssh;
in {
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
          forwardAgent = true;
          serverAliveInterval = 600;
          setEnv = {
            "LC_ALL" = "C";
          };
        };
        "services" = {
          hostname = "services";
          user = "mm";
          serverAliveInterval = 600;
          setEnv = {
            "LC_ALL" = "C";
          };
        };
        "proxmox" = {
          hostname = "proxmox";
          user = "root";
          setEnv = {
            "LC_ALL" = "C";
          };
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
