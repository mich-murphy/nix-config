# underclass (github:ghuntley/underclass) pools Claude Code, ChatGPT/Codex and
# Copilot subscriptions behind one local endpoint on 127.0.0.1:8080. Each host
# runs its own pool: work accounts on dev-01, personal accounts on the Macs.
# Accounts are added in its web UI and stored in ~/.local/share/underclass/pool.db.
#
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

  # The previous arrangement sourced keys from this file. underclass ignores
  # it and would mint new keys, so refuse to switch until they are moved.
  home.activation.underclassLegacyEnv = lib.hm.dag.entryBefore [ "writeBoundary" ] ''
    if [[ -e ${lib.escapeShellArg "${home}/.config/underclass/env"} ]]; then
      errorEcho "~/.config/underclass/env is no longer read: move its keys into ~/.config/underclass/config.toml (see the README), then delete it"
      exit 1
    fi
  '';

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
