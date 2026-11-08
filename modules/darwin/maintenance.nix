{
  # Determinate Nixd collects garbage automatically but does not prune
  # generations, so expire system generations older than 14 days and let its
  # GC free them. Embedded Home Manager generations live inside each system
  # generation and need no separate pruning.
  launchd.daemons.nix-wipe-history.serviceConfig = {
    ProgramArguments = [
      "/nix/var/nix/profiles/default/bin/nix"
      "profile"
      "wipe-history"
      "--profile"
      "/nix/var/nix/profiles/system"
      "--older-than"
      "14d"
    ];
    StartCalendarInterval = [
      {
        Weekday = 7;
        Hour = 9;
        Minute = 0;
      }
    ];
    StandardOutPath = "/var/log/nix-wipe-history.log";
    StandardErrorPath = "/var/log/nix-wipe-history.log";
  };
}
