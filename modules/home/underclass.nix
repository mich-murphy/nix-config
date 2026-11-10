# underclass (github:ghuntley/underclass) pools Claude Code, ChatGPT/Codex and
# Copilot subscriptions behind one local endpoint. Each host runs its own pool:
# work accounts on dev-01, personal accounts on the Macs. Accounts are added in
# its web UI and stored in ~/.local/share/underclass/pool.db.
#
# Keys live in ~/.config/underclass/env, a local file that is never tracked:
#   UNDERCLASS_PROXY_KEY=...  sent by clients
#   UNDERCLASS_UI_TOKEN=...   for the web UI and admin API
# Without the file, underclass mints both into pool.db on first start.
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
  bind = "127.0.0.1:8080";
  envFile = "${config.home.homeDirectory}/.config/underclass/env";
  # launchd has no EnvironmentFile, so export the file's variables first.
  serve = pkgs.writeShellScript "underclass-serve" ''
    set -a
    if [[ -r ${lib.escapeShellArg envFile} ]]; then
      . ${lib.escapeShellArg envFile}
    fi
    exec ${lib.getExe package} serve --bind ${bind}
  '';
in
{
  # underclass and utop, its live pool monitor.
  home.packages = [ package ];

  launchd.agents.underclass = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [ "${serve}" ];
      KeepAlive = true;
      RunAtLoad = true;
      StandardOutPath = "${config.home.homeDirectory}/Library/Logs/underclass.log";
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/underclass.log";
    };
  };

  systemd.user.services.underclass = lib.mkIf isLinux {
    Unit = {
      Description = "underclass subscription pool proxy";
      After = [ "network-online.target" ];
      Wants = [ "network-online.target" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} serve --bind ${bind}";
      # The leading "-" lets the service start before the file exists.
      EnvironmentFile = "-${envFile}";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };
}
