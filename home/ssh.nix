{...}: {
  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    settings = {
      "ai-dev" = {
        User = "michael";
        LocalForward = [
          {
            bind = {
              address = "127.0.0.1";
              port = 19432;
            };
            host = {
              address = "127.0.0.1";
              port = 19432;
            };
          }
        ];
        ExitOnForwardFailure = true;
      };

      # configure 1password ssh agent
      "*" = {
        IdentityAgent = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
        HashKnownHosts = true;
      };
    };
  };
}
