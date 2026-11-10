# Optional keys live in ~/.config/underclass/config.toml, a local file that is
# never tracked and that underclass itself reads on every platform:
#   proxy_key = "..."   sent by clients
#   ui_token = "..."    for the web UI and admin API
# Without them, underclass mints both into pool.db on first start.
{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux system;
  package = inputs.underclass.packages.${system}.underclass;
  home = config.home.homeDirectory;
in
{
  # underclass and utop, its live pool monitor.
  home.packages = [ package ];

  launchd.agents.underclass = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe package)
        "serve"
      ];
      KeepAlive = true;
      RunAtLoad = true;
      StandardOutPath = "${home}/Library/Logs/underclass.log";
      StandardErrorPath = "${home}/Library/Logs/underclass.log";
    };
  };

  systemd.user.services.underclass = lib.mkIf isLinux {
    Unit = {
      Description = "underclass subscription pool proxy";
      After = [ "network-online.target" ];
      Wants = [ "network-online.target" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} serve";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };
}
