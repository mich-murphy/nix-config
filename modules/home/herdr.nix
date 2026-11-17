{
  config,
  lib,
  pkgs,
  ...
}:
let
  herdr = lib.getExe pkgs.herdr;
  # Herdr's integrations write into each agent's own config: Pi's reports
  # idle, working and blocked state, which Herdr otherwise guesses from the
  # screen; Claude Code's and Codex's report the session Herdr resumes after
  # a server restart.
  integrations = [
    "pi"
    "claude"
    "codex"
  ];
  # Claude Code's and Codex's installs edit settings files that Home Manager
  # merges into, so they run after those merges.
  predecessors = [
    "linkGeneration"
    "claudeCodeSettings"
    "piCodingAgentMutableSettings"
    "codexMutableSettings"
  ];
in
{
  # Home Manager's DAG silently ignores missing predecessor names.
  assertions = map (name: {
    assertion = builtins.hasAttr name config.home.activation;
    message = "herdr.nix: required activation predecessor '${name}' is missing from home.activation";
  }) predecessors;

  home.packages = [ pkgs.herdr ];

  xdg.configFile."herdr/config.toml".source = config.lib.nixConfig.file "config/herdr/config.toml";

  home.activation.herdrIntegrations = lib.hm.dag.entryAfter predecessors ''
    herdrStatus=$(${herdr} integration status 2>/dev/null || true)
    for target in ${lib.concatStringsSep " " integrations}; do
      if ! grep -q "^$target: current " <<<"$herdrStatus"; then
        run ${herdr} integration install "$target" \
          || echo "herdr integration install $target failed; rerun it by hand" >&2
      fi
    done
  '';
}
