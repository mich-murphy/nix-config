{
  lib,
  config,
  ...
}: let
  cfg = config.common.ssh;
in {
  options.common.ssh = {
    enable = lib.mkEnableOption "Enable SSH config with configured hosts";
  };

  config = lib.mkIf cfg.enable {
    programs.ssh = {
      enable = true;
      enableDefaultConfig = false;
      matchBlocks = {
        # configure 1password ssh agent
        "*" = {
          extraOptions = {
            "IdentityAgent" = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
          };
        };
        "proxmox" = {
          hostname = "proxmox";
          user = "root";
          setEnv = {
            "LC_ALL" = "C";
          };
        };
      };
    };
  };
}
