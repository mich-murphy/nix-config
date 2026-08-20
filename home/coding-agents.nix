{
  config,
  lib,
  pkgs,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  piExtensionSources = ../config/agents/extensions;
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
  piExtensions = pkgs.linkFarm "pi-extensions" [
    {
      name = "README.md";
      path = piExtensionSources + "/README.md";
    }
    {
      name = "herdr-agent-state.ts";
      path = piExtensionSources + "/herdr-agent-state.ts";
    }
    {
      name = "moshi-hooks.ts";
      path = piExtensionSources + "/moshi-hooks.ts";
    }
    {
      name = "claude-sdk-provider";
      path = packagePiExtension {
        directory = "claude-sdk-provider";
        npmDepsHash = "sha256-ep5H2levq+9BUi7ihSzUpXb0iAiYTgC5Pw3e51Pb0XA=";
      };
    }
    {
      name = "pi-skill-toggle";
      path = piExtensionSources + "/pi-skill-toggle";
    }
    {
      name = "pi-web-tools";
      path = packagePiExtension {
        directory = "pi-web-tools";
        npmDepsHash = "sha256-RKSaPsQMsPErKFUogSIzHawWCkiepBPD+r0B0pq25hU=";
      };
    }
  ];
in {
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/skills".source = liveLink "${agentConfig}/skills";
    ".pi/agent/extensions" = {
      source = piExtensions;
      force = true;
    };
  };
}
