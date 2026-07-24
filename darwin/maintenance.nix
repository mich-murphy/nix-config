{...}: {
  # nix.gc requires nix.enable, so these agents retain Determinate ownership
  # while pruning both system and Home Manager generations.
  launchd.daemons.nix-gc.serviceConfig = {
    ProgramArguments = [
      "/bin/sh"
      "-lc"
      "/nix/var/nix/profiles/default/bin/nix profile wipe-history --profile /nix/var/nix/profiles/system --older-than 14d && exec /nix/var/nix/profiles/default/bin/nix store gc"
    ];
    StartCalendarInterval = [
      {
        Weekday = 7;
        Hour = 9;
      }
    ];
    StandardOutPath = "/tmp/nix-gc.log";
    StandardErrorPath = "/tmp/nix-gc.log";
  };

  launchd.user.agents.nix-gc-user.serviceConfig = {
    ProgramArguments = [
      "/bin/sh"
      "-lc"
      "for p in $HOME/.local/state/nix/profiles/*; do case $p in *-link) continue;; esac; /nix/var/nix/profiles/default/bin/nix profile wipe-history --profile \"$p\" --older-than 14d; done"
    ];
    StartCalendarInterval = [
      {
        Weekday = 7;
        Hour = 8;
      }
    ];
    StandardOutPath = "/tmp/nix-gc-user.log";
    StandardErrorPath = "/tmp/nix-gc-user.log";
  };
}
