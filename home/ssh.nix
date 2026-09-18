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
        Port = 22;
        IdentityFile = "~/.ssh/macbook";
        IdentitiesOnly = true;
      };

      # DigitalOcean droplet (github.com/builtgrid/droplets). Droplets have
      # no DNS name and this IP is ephemeral: update HostName from
      # `HOST=dev-01 just apply`'s ssh_connection_string output after every
      # rebuild. The alias matches the ssh column of the droplets justfile's
      # host_inventory - keep them in step.
      "bgd-01" = {
        HostName = "170.64.193.123";
        User = "dev";
        Port = 22;
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
