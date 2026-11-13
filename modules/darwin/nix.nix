{
  # Determinate installs Nix; its module stops nix-darwin managing it.
  determinateNix = {
    enable = true;
    # Determinate Nixd is the only collector. The job below only expires
    # generations, which its disk-pressure collector does not do
    # (https://github.com/DeterminateSystems/determinate/issues/169).
    determinateNixd.garbageCollector.strategy = "automatic";
  };

  # The uninstaller embeds a second, uncached darwin-system build;
  # `nix run nix-darwin#darwin-uninstaller` does the same job when needed.
  system.tools.darwin-uninstaller.enable = false;

  # The module takes over the installer's nix.custom.conf. Allow
  # replacing only the untouched installer defaults (hashes from
  # nix-darwin's modules/nix); an edited file still aborts.
  environment.etc."nix/nix.custom.conf".knownSha256Hashes = [
    "6787fade1cf934f82db554e78e1fc788705c2c5257fddf9b59bdd963ca6fec63" # DetSys v0.33.0
    "3bd68ef979a42070a44f8d82c205cfd8e8cca425d91253ec2c10a88179bb34aa" # DetSys v0.34.0
  ];

  launchd.daemons.nix-wipe-history.serviceConfig = {
    ProgramArguments = [
      "/nix/var/nix/profiles/default/bin/nix"
      "profile"
      "wipe-history"
      "--profile"
      "/nix/var/nix/profiles/system"
      "--older-than"
      "7d"
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
