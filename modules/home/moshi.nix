# Moshi's hook daemon. `moshi-hook install` writes the path it runs from into
# every Claude, Codex and Pi hook, and Codex's approval covers that command, so
# hooks name a path that survives version bumps. The service runs the store
# path, so a new version restarts it. `moshi-hook` and `moshi` on PATH run the
# stable copy, so a manual `moshi-hook install` writes the same path.
{
  config,
  lib,
  osConfig ? null,
  pkgs,
  ...
}:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
  exe = lib.getExe pkgs.moshi-hook;
  # On the Macs nix-darwin installs the signed app bundle
  # (pkgs.moshi-hook.app in modules/darwin/applications.nix) here. Hooks and
  # Codex's approval name this path, so do not change it; on Linux the
  # activation below copies the binary instead.
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
  serveArgs = [
    "serve"
    "--gateway-listen"
    "127.0.0.1:24543"
  ];
  # Services get a minimal PATH. Give the daemon a shell's PATH so programs it
  # runs by name (herdr, git, gh, tmux) resolve the same way.
  servicePath = lib.concatStringsSep ":" (
    config.home.sessionPath
    ++ [ "${config.home.profileDirectory}/bin" ]
    ++ lib.optionals isDarwin [
      "/run/current-system/sw/bin"
      "/opt/homebrew/bin"
    ]
    ++ [
      "/nix/var/nix/profiles/default/bin"
      "/usr/local/bin"
      "/usr/bin"
      "/bin"
      "/usr/sbin"
      "/sbin"
    ]
  );
  wrapper =
    name:
    pkgs.writeShellApplication {
      inherit name;
      text = ''exec ${lib.escapeShellArg hookPath} "$@"'';
    };
  # `install` restarts running agents when it changes their hooks, so it only
  # runs when `status` reports them out of date. A service that has just been
  # started needs a moment to open the socket `install` talks to.
  ensureHooks = pkgs.writeShellApplication {
    name = "moshi-ensure-hooks";
    runtimeInputs = [ pkgs.jq ];
    text = ''
      moshi=${lib.escapeShellArg hookPath}
      if [[ ! -x $moshi ]]; then
        echo "Moshi hooks not checked: $moshi is missing" >&2
        exit 0
      fi
      status=$("$moshi" status --json || true)
      if jq -e --argjson targets ${lib.escapeShellArg (builtins.toJSON targets)} \
        'all(.hooks[] | select(.target | IN($targets[])); .status == "current")' \
        <<<"$status" >/dev/null; then
        exit 0
      fi
      socket=$(jq -r '.socketPath // empty' <<<"$status" || true)
      for _ in {1..20}; do
        [[ -S $socket ]] && break
        sleep 0.5
      done
      MOSHI_HOOK_SKIP_FIRST_RUN=1 "$moshi" install --target ${lib.concatStringsSep "," targets} \
        || echo "moshi-hook install failed; rerun it by hand" >&2
    '';
  };
in
{
  assertions = lib.optional (isDarwin && osConfig != null) {
    assertion = builtins.elem pkgs.moshi-hook.app osConfig.environment.systemPackages;
    message = "moshi.nix: hooks name the app bundle that nix-darwin installs from pkgs.moshi-hook.app, which is missing from environment.systemPackages";
  };

  home.packages = map wrapper [
    "moshi-hook"
    "moshi"
  ];

  launchd.agents.moshi-hook = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [ exe ] ++ serveArgs;
      EnvironmentVariables.PATH = servicePath;
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
      ExecStart = lib.escapeShellArgs ([ exe ] ++ serveArgs);
      Environment = "PATH=${servicePath}";
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

    # After the settings merges, which keep Moshi's undeclared keys, and after
    # the service starts (setupLaunchAgents on the Macs, reloadSystemd on
    # Linux), so `install` finds the daemon's socket.
    moshiHooks = lib.hm.dag.entryAfter [
      "linkGeneration"
      "moshiHookCopy"
      "claudeCodeSettings"
      "piCodingAgentMutableSettings"
      "setupLaunchAgents"
      "reloadSystemd"
    ] "run ${lib.getExe ensureHooks}";
  };
}
