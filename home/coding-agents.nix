{
  config,
  lib,
  pkgs,
  repoRoot,
  ...
}: let
  agentConfig = "${repoRoot}/config/agents";
  agentConfigSources = ../config/agents;
  piExtensionSources = agentConfigSources + "/extensions";
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
  home.file = {
    ".agents/skills".source = liveLink "${agentConfig}/skills";
    ".claude/skills".source = liveLink "${agentConfig}/skills";
    ".pi/agent/AGENTS.md".source = agentConfigSources + "/AGENTS.md";
    ".pi/agent/agents/Explore.md".source = agentConfigSources + "/agents/Explore.md";
    # Manage repository-owned entries individually so external integration
    # installers can add and update their own files in the containing directory.
    ".pi/agent/extensions/README.md".source = piExtensionSources + "/README.md";
    ".pi/agent/extensions/claude-sdk-provider".source = claudeSdkProvider;
    ".pi/agent/extensions/no-sleep".source = piExtensionSources + "/no-sleep";
    ".pi/agent/extensions/pi-skill-toggle".source = piExtensionSources + "/pi-skill-toggle";
    ".pi/agent/extensions/pi-vim".source = piExtensionSources + "/pi-vim";
  };
}
