{ lib, ... }:
let
  detsysInstallerNixCustomConfSha256 = {
    "v0.33.0" = "6787fade1cf934f82db554e78e1fc788705c2c5257fddf9b59bdd963ca6fec63";
    "v0.34.0" = "3bd68ef979a42070a44f8d82c205cfd8e8cca425d91253ec2c10a88179bb34aa";
  };
in
{
  determinateNix = {
    enable = true;
    determinateNixd.garbageCollector.strategy = "automatic";
    customSettings = {
      substituters = [
        "https://cache.nixos.org/?priority=30"
        "https://cache.numtide.com?priority=40"
      ];
      extra-trusted-public-keys = [
        "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
      ];
    };
  };

  system.tools.darwin-uninstaller.enable = false;

  environment.etc."nix/nix.custom.conf".knownSha256Hashes =
    lib.attrValues detsysInstallerNixCustomConfSha256;

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
