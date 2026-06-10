{...}: {
  # nix configuration
  # reference: https://daiderd.com/nix-darwin/manual/index.html#sec-options

  imports = [
    ./apps.nix
    ./system.nix
    ./user.nix
  ];

  # create /etc/zshrc which loads nix-darwin environment
  # required if you want to use darwin default shell (zsh)
  programs.zsh.enable = true;
  programs.fish.enable = true;

  nixpkgs = {
    config.allowUnfree = true; # allow unfree packages
    hostPlatform = "aarch64-darwin";
  };

  # nix-darwin management disabled in place of Determinate
  nix.enable = false;

  environment.variables = {
    LESS = "--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4";
  };

  # nix-darwin's nix.gc.* launchd integration requires nix.enable = true, but
  # this host leaves Nix managed by the Determinate installer.
  # Runs as a daemon (root) so it can prune system profile generations,
  # which are GC roots — without this, gc reclaims almost nothing.
  launchd.daemons.nix-gc = {
    serviceConfig = {
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
  };

  system.stateVersion = 4;
}
