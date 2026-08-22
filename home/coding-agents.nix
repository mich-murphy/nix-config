{
  config,
  lib,
  pkgs,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  piExtensionSources = ../config/agents/extensions;
  piPackages = [
    "npm:pi-subagents"
    "npm:pi-web-access"
  ];
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
  packagePiExtension = {
    directory,
    npmDepsHash,
  }: let
    src = piExtensionSources + "/${directory}";
    manifest = lib.importJSON (src + "/package.json");
    package = pkgs.buildNpmPackage {
      pname = manifest.name;
      inherit (manifest) version;
      inherit src npmDepsHash;
      dontNpmBuild = true;
      # Pi supplies peer packages at runtime; keep only the extension's runtime
      # dependency closure in the Nix package.
      postPatch = ''
        ${lib.getExe pkgs.jq} 'del(.devDependencies, .peerDependencies)' package.json > package.json.tmp
        mv package.json.tmp package.json
        ${lib.getExe pkgs.jq} '
          .packages[""] |= del(.devDependencies, .peerDependencies)
          | .packages |= with_entries(select(.key == "" or (.value.dev // false | not)))
        ' package-lock.json > package-lock.json.tmp
        mv package-lock.json.tmp package-lock.json
      '';
    };
  in "${package}/lib/node_modules/${manifest.name}";
  claudeSdkProvider = packagePiExtension {
    directory = "claude-sdk-provider";
    npmDepsHash = "sha256-ep5H2levq+9BUi7ihSzUpXb0iAiYTgC5Pw3e51Pb0XA=";
  };
in {
  # Third-party packages remain owned by Pi so their manifests, bundled skills,
  # runtime dependencies, and updates keep working as designed. Home Manager
  # only reconciles missing package registrations.
  home.activation.installPiPackages = lib.hm.dag.entryAfter ["writeBoundary"] ''
    piBin="$(command -v pi || true)"
    if [[ -z "$piBin" && -x /opt/homebrew/bin/pi ]]; then
      piBin=/opt/homebrew/bin/pi
    fi

    if [[ -n "$piBin" ]]; then
      settings="$HOME/.pi/agent/settings.json"
      ${lib.concatMapStringsSep "\n" (package: ''
        if [[ ! -f "$settings" ]] || ! ${lib.getExe pkgs.jq} -e --arg source ${lib.escapeShellArg package} '
          (.packages // []) | any(
            if type == "string" then . == $source else .source == $source end
          )
        ' "$settings" >/dev/null; then
          run "$piBin" install ${lib.escapeShellArg package}
        fi
      '')
      piPackages}
      unset settings
    else
      echo "Pi is not installed; skipping Pi package reconciliation" >&2
    fi
    unset piBin
  '';

  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/skills".source = liveLink "${agentConfig}/skills";
    # Manage repository-owned entries individually so external integration
    # installers can add and update their own files in the containing directory.
    ".pi/agent/extensions/README.md".source = piExtensionSources + "/README.md";
    ".pi/agent/extensions/claude-sdk-provider".source = claudeSdkProvider;
    ".pi/agent/extensions/pi-skill-toggle".source = piExtensionSources + "/pi-skill-toggle";
  };
}
