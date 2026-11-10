# Moshi's hook daemon. `moshi-hook install` writes the path it runs from into
# every Claude, Codex and Pi hook, and Codex's approval covers that command.
# Hooks therefore name a path that survives version bumps: nix-darwin's copy in
# /Applications/Nix Apps on the Macs (see modules/darwin/applications.nix), a
# copy in ~/.local/libexec on Linux. `moshi-hook` and `moshi` on PATH run the
# stable copy, so a manual `moshi-hook install` writes the same path.
#
# On the Macs the service runs the stable copy too: running a binary from
# inside the store's app bundle tags the bundle with com.apple.macl, and macOS
# then stops nix-daemon from deleting it, which fails garbage collection.
# nix-darwin copies the app before Home Manager activates, and the version in
# the agent's environment changes its plist, so a new version restarts it. On
# Linux the service runs the store path, which restarts it the same way.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin isLinux;
  exe = lib.getExe pkgs.moshi-hook;
  # Services get a minimal PATH. Give the daemon a shell's PATH so programs it
  # runs by name (herdr, git, gh, tmux) resolve the same way.
  servicePath = lib.concatStringsSep ":" (
    [
      "${config.home.homeDirectory}/.local/bin"
      "${config.home.profileDirectory}/bin"
    ]
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
  wrapper =
    name:
    pkgs.writeShellApplication {
      inherit name;
      text = ''exec ${lib.escapeShellArg hookPath} "$@"'';
    };
in
{
  home.packages = map wrapper [
    "moshi-hook"
    "moshi"
  ];

  launchd.agents.moshi-hook = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [
        hookPath
        "serve"
      ];
      EnvironmentVariables = {
        PATH = servicePath;
        MOSHI_HOOK_VERSION = pkgs.moshi-hook.version;
      };
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

    # Reinstall only when Moshi reports its hooks out of date (new install,
    # moved path or a release that changes them). Runs after the settings
    # merges, which keep Moshi's keys because they are not declared, and after
    # the service starts (setupLaunchAgents on the Macs, reloadSystemd on
    # Linux), so `install` finds the daemon's socket.
    moshiHooks =
      lib.hm.dag.entryAfter
        [
          "linkGeneration"
          "moshiHookCopy"
          "claudeCodeSettings"
          "piCodingAgentMutableSettings"
          "setupLaunchAgents"
          "reloadSystemd"
        ]
        ''
          moshi=${lib.escapeShellArg hookPath}
          if [[ ! -x $moshi ]]; then
            warnEcho "Moshi hooks not checked: $moshi is missing"
          elif ! "$moshi" status --json | ${lib.getExe pkgs.jq} -e \
              '[.hooks[] | select(.target | IN(${
                lib.concatMapStringsSep ", " (t: ''"${t}"'') targets
              })) | .status == "current"] | all' >/dev/null; then
            # A just-started service needs a moment to open its socket.
            socket=$("$moshi" status --json | ${lib.getExe pkgs.jq} -r .socketPath) || socket=
            for _ in {1..20}; do [[ -S $socket ]] && break; sleep 0.5; done
            run env MOSHI_HOOK_SKIP_FIRST_RUN=1 "$moshi" install --target ${lib.concatStringsSep "," targets} \
              || warnEcho "moshi-hook install failed; rerun it by hand"
          fi
        '';
  };
}
