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
      matchBlocks = {
        # configure 1password ssh agent
        "*" = {
          extraOptions = {
            "IdentityAgent" = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
          };
        };
        "media" = {
          hostname = "media";
          user = "mm";
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
      };
    };
  };
}
