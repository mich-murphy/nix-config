# Moshi's hook daemon. `moshi-hook install` writes the path it runs from into
# every Claude, Codex and Pi hook, and Codex's approval covers that command.
# Hooks therefore name a path that survives version bumps: nix-darwin's copy in
# /Applications/Nix Apps on the Macs (see modules/darwin/applications.nix), a
# copy in ~/.local/libexec on Linux. The service runs the store path, so a new
# version restarts it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
  exe = lib.getExe pkgs.moshi-hook;
  # Services get a minimal PATH; Moshi reads this to reach herdr panes.
  herdrPath = lib.getExe pkgs.herdr;
  hookPath =
    if isDarwin then
      "/Applications/Nix Apps/MoshiHook.app/Contents/MacOS/moshi-hook"
    else
      "${config.home.homeDirectory}/.local/libexec/moshi-hook";
  targets = [
    "claude"
    "codex"
    "pi"
  ];
in
{
  launchd.agents.moshi-hook = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [
        exe
        "serve"
      ];
      EnvironmentVariables.MOSHI_HERDR_PATH = herdrPath;
      KeepAlive = true;
      RunAtLoad = true;
      StandardOutPath = "${config.home.homeDirectory}/Library/Logs/moshi-hook.log";
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/moshi-hook.log";
    };
  };

  systemd.user.services.moshi-hook = lib.mkIf isLinux {
    Unit = {
      Description = "Moshi agent-hooks daemon";
      After = [ "network-online.target" ];
      Wants = [ "network-online.target" ];
    };
    Service = {
      ExecStart = "${exe} serve --gateway-listen 127.0.0.1:24543";
      Environment = "MOSHI_HERDR_PATH=${herdrPath}";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };

  home.activation = {
    moshiHookCopy = lib.mkIf isLinux (
      lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        if ! cmp -s ${exe} ${lib.escapeShellArg hookPath}; then
          run install -D -m 755 ${exe} ${lib.escapeShellArg hookPath}
        fi
      ''
    );

    # Reinstall only when Moshi reports its hooks out of date (new install,
    # moved path or a release that changes them). Runs after the settings
    # merges, which keep Moshi's keys because they are not declared.
    moshiHooks =
      lib.hm.dag.entryAfter
        [
          "linkGeneration"
          "moshiHookCopy"
          "claudeCodeSettings"
          "piCodingAgentMutableSettings"
        ]
        ''
          moshi=${lib.escapeShellArg hookPath}
          if [[ ! -x $moshi ]]; then
            warnEcho "Moshi hooks not checked: $moshi is missing"
          elif ! "$moshi" status --json | ${lib.getExe pkgs.jq} -e \
              '[.hooks[] | select(.target | IN(${
                lib.concatMapStringsSep ", " (t: ''"${t}"'') targets
              })) | .status == "current"] | all' >/dev/null; then
            run env MOSHI_HOOK_SKIP_FIRST_RUN=1 "$moshi" install --target ${lib.concatStringsSep "," targets} \
              || warnEcho "moshi-hook install failed; rerun it by hand"
          fi
        '';
  };
}
