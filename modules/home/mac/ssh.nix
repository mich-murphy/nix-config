{ lib, ... }:
let
  onePasswordPublicKeys = {
    dev-01 = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPKulgPiyvlyoTEC9rfhFoIofPPxiFvU5UrGndDA85L3 dev-01";
    docker-host = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIH1TgAtlovn+B5ojfw7JRFDi8UxcTkHym30wEg6jekF docker-host";
    github = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHvs4jPIjoGP3YZJfJrk5+RFD50SCEQKSXvo5q7SYxQS GitHub - mich-murphy";
    github_bc = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILeCwvXQKiDs6E4/uJdk8rnT2fbSZTP5bpx/nqyCrIVV GitHub - BusinessCraft";
    truenas = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIETvpceFBvOQy8fC0gIbsPDrZirDHK+KdNjX4mplCw5/ truenas";
  };
in
{
  home.file = lib.mapAttrs' (
    name: key: lib.nameValuePair ".ssh/${name}.pub" { text = "${key}\n"; }
  ) onePasswordPublicKeys;

  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    settings = {
      "dev-01" = {
        HostName = "bsncraft-dev-01.australiaeast.cloudapp.azure.com";
        User = "dev";
        IdentityFile = "~/.ssh/dev-01.pub";
        ControlMaster = "auto";
        ControlPath = "~/.ssh/cm-%C";
        ControlPersist = "10m";
      };

      "docker-host" = {
        User = "mm";
        IdentityFile = "~/.ssh/docker-host.pub";
      };

      "truenas" = {
        HostName = "10.77.20.101";
        User = "truenas_admin";
        ProxyJump = "docker-host";
        IdentityFile = "~/.ssh/truenas.pub";
      };

      "Match host github.com,ssh.github.com tagged github-bc" = {
        User = "git";
        IdentityFile = "~/.ssh/github_bc.pub";
      };
      "Match host github.com,ssh.github.com !tagged github-bc" = {
        User = "git";
        IdentityFile = "~/.ssh/github.pub";
      };

      "*" = {
        IdentityAgent = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
        IdentitiesOnly = true;
        IdentityFile = "none";
        ForwardAgent = false;
        HashKnownHosts = true;
        ServerAliveInterval = 15;
        ServerAliveCountMax = 4;
      };
    };
  };
}
