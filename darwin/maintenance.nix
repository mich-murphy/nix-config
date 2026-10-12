{...}: {
  # nix.gc requires nix.enable, so this daemon expires old system generations
  # while Determinate keeps ownership of Nix. Embedded Home Manager generations
  # live inside each system generation and need no separate pruning.
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
}
