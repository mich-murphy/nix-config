{
  config,
  lib,
  pkgs,
  ...
}:
let
  log = "${config.home.homeDirectory}/Library/Logs/nixcache.log";
in
{
  launchd.agents.nixcache = {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe pkgs.nixcache)
        "-foreground"
        "-cache-upstream=off"
        "-endpoint"
        "https://ord.storage.namespaceapis.com"
        "-listen"
        "127.0.0.1:18765"
      ];
      RunAtLoad = true;
      KeepAlive = true;
      StandardOutPath = log;
      StandardErrorPath = log;
    };
  };
}
