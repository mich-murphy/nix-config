{...}: {
  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    settings = {
      # pin the key; the 1password agent otherwise offers every key and the
      # server disconnects after 6 failures
      "ai-dev" = {
        User = "michael";
        IdentityFile = "~/.ssh/ai-dev.pub";
        IdentitiesOnly = true;
      };

      "bc-01" = {
        HostName = "bsncraft-dev-01.australiaeast.cloudapp.azure.com";
        User = "dev";
        IdentityFile = "~/.ssh/macbook";
        IdentitiesOnly = true;
      };

      # configure 1password ssh agent
      "*" = {
        IdentityAgent = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
        HashKnownHosts = true;
      };
    };
  };
}
