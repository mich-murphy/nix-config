{ lib, ... }:
let
  # Public halves only; the private keys stay in 1Password. With
  # IdentitiesOnly, ssh offers only agent keys that match an IdentityFile, and
  # a .pub file is enough to match. Comments are the 1Password item names.
  publicKeys = {
    # "dev-01"
    dev-01 = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPKulgPiyvlyoTEC9rfhFoIofPPxiFvU5UrGndDA85L3 dev-01";
    # "docker-host"
    docker-host = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIH1TgAtlovn+B5ojfw7JRFDi8UxcTkHym30wEg6jekF docker-host";
    # "GitHub - mich-murphy"
    github = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHvs4jPIjoGP3YZJfJrk5+RFD50SCEQKSXvo5q7SYxQS GitHub - mich-murphy";
    # "GitHub - BusinessCraft"
    github_bc = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILeCwvXQKiDs6E4/uJdk8rnT2fbSZTP5bpx/nqyCrIVV GitHub - BusinessCraft";
    # "truenas"
    truenas = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIETvpceFBvOQy8fC0gIbsPDrZirDHK+KdNjX4mplCw5/ truenas";
  };
in
{
  home.file = lib.mapAttrs' (
    name: key: lib.nameValuePair ".ssh/${name}.pub" { text = "${key}\n"; }
  ) publicKeys;

  # ssh takes the first value it finds for each option, so specific hosts come
  # first. Home Manager always writes "*" last. IdentityFile is the exception:
  # it accumulates, which is why "*" can add `none` without dropping a host's key.
  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    settings = {
      "dev-01" = {
        HostName = "bsncraft-dev-01.australiaeast.cloudapp.azure.com";
        User = "dev";
        IdentityFile = "~/.ssh/dev-01.pub";
        # Reuse one authenticated connection, so ssh, deploy-rs and scp to
        # dev-01 ask 1Password for approval once rather than on every
        # connection. Herdr --remote runs its own master.
        ControlMaster = "auto";
        ControlPath = "~/.ssh/cm-%C";
        ControlPersist = "10m";
      };

      # Over the tailnet (home-infra docs/homelab-roadmap-handover.md).
      "docker-host" = {
        User = "mm";
        IdentityFile = "~/.ssh/docker-host.pub";
      };

      # Not routable from the Mac directly; reached through docker-host.
      "truenas" = {
        HostName = "10.77.20.101";
        User = "truenas_admin";
        ProxyJump = "docker-host";
        IdentityFile = "~/.ssh/truenas.pub";
      };

      # Personal account. Git inside ~/businesscraft/ puts the work key in
      # front of this one with -i (git.nix), and GitHub signs in as whichever
      # key it accepts first. No ControlMaster here: a shared connection would
      # carry the first account's login into the other account's repositories.
      "github.com" = {
        User = "git";
        IdentityFile = "~/.ssh/github.pub";
      };

      "*" = {
        IdentityAgent = ''"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"'';
        # Offer only the key a host names. Without this, ssh offers all
        # seven 1Password keys plus the default files to every host, which
        # exceeds sshd's MaxAuthTries of 6 and tells every server you connect
        # to which keys you hold. A host with no entry is offered no key;
        # add an entry, or pass `-i ~/.ssh/<key>.pub`.
        IdentitiesOnly = true;
        IdentityFile = "none";
        ForwardAgent = false;
        HashKnownHosts = true;
        # Drop dead connections (sleep, network change) after about a minute
        # rather than hanging. Herdr's own fallbacks are the same values; ours
        # would win over them, so they are kept equal.
        ServerAliveInterval = 15;
        ServerAliveCountMax = 4;
      };
    };
  };
}
