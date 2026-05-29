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
      settings = {
        # configure 1password ssh agent
        "*" = {
          IdentityAgent = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
          HashKnownHosts = true;
        };
        "proxmox" = {
          HostName = "proxmox";
          User = "root";
          SetEnv = {
            LC_CTYPE = "C";
            LC_COLLATE = "C";
            LANG = "C";
          };
        };
      };
    };
  };
}
