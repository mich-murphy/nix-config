{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  herdr = lib.getExe pkgs.herdr;
  jq = lib.getExe pkgs.jq;
  annotate = pkgs.herdr-plugin-annotate;
  tokyonight = "${inputs.tokyonight}/extras";
  piColors = (lib.importJSON "${tokyonight}/pi/tokyonight_night.json").colors;
  tokyonightPalette = (lib.importJSON "${tokyonight}/opencode/tokyonight_night.json").defs;
  tokyonightBlend =
    fg: alpha: bg:
    let
      channel = hex: i: lib.fromHexString (builtins.substring (1 + 2 * i) 2 hex);
      mix = i: builtins.floor (alpha * channel fg i + (1 - alpha) * channel bg i + 0.5);
      hex2 = n: lib.toLower (lib.fixedWidthString 2 "0" (lib.toHexString n));
    in
    "#"
    + lib.concatMapStrings (i: hex2 (mix i)) [
      0
      1
      2
    ];
  plannotatorTuiConfig = (pkgs.formats.toml { }).generate "plannotator-tui-config.toml" {
    ui = {
      theme = "dark";
      code_theme = "${tokyonight}/sublime/tokyonight_night.tmTheme";
    };
    colors = {
      accent = piColors.accent;
      heading = piColors.mdHeading;
      comment = piColors.warning;
      looks-good = piColors.success;
      delete = piColors.error;
      comment-bg = tokyonightBlend piColors.warning 0.3 tokyonightPalette.bg;
      looks-good-bg = tokyonightBlend piColors.success 0.3 tokyonightPalette.bg;
      block = piColors.toolPendingBg;
      toolbar = tokyonightPalette.fg_gutter;
      cursor = tokyonightPalette.dark3;
      selection = piColors.selectedBg;
      code = piColors.mdCode;
      code-block = "reset";
      link = piColors.mdLink;
      quote = piColors.mdQuote;
      border = piColors.dim;
    };
  };
  integrations = [
    "pi"
    "claude"
    "codex"
  ];
  agentSettingsMergePredecessors = [
    "linkGeneration"
    "claudeCodeSettings"
    "piCodingAgentMutableSettings"
    "codexMutableSettings"
  ];
in
{
  assertions = map (name: {
    assertion = builtins.hasAttr name config.home.activation;
    message = "herdr.nix: required activation predecessor '${name}' is missing from home.activation";
  }) agentSettingsMergePredecessors;

  home.packages = [ pkgs.herdr ];

  xdg.configFile."herdr/config.toml".source = config.lib.nixConfig.file "config/herdr/config.toml";
  xdg.configFile."plannotator-tui/config.toml".source = plannotatorTuiConfig;

  xdg.dataFile."herdr-plugins/annotate".source = annotate;

  home.activation.herdrIntegrations = lib.hm.dag.entryAfter agentSettingsMergePredecessors ''
    herdrStatus=$(${herdr} integration status 2>/dev/null || true)
    for target in ${lib.concatStringsSep " " integrations}; do
      if ! grep -q "^$target: current " <<<"$herdrStatus"; then
        run ${herdr} integration install "$target" \
          || echo "herdr integration install $target failed; rerun it by hand" >&2
      fi
    done
  '';

  home.activation.herdrAnnotatePlugin = lib.hm.dag.entryAfter [ "herdrIntegrations" ] ''
    annotateWant=${annotate}
    annotateNow=$(${herdr} plugin list --json --plugin annotate 2>/dev/null \
      | ${jq} -r '.result.plugins[0] // {} | "\(.plugin_root // "") \(.enabled != false)"' \
      || true)
    read -r annotateRoot annotateEnabled <<<"$annotateNow" || true
    if [ "$annotateRoot" != "$annotateWant" ]; then
      annotateFlag=--enabled
      [ "$annotateEnabled" = false ] && annotateFlag=--disabled
      run ${herdr} plugin link "$annotateWant" "$annotateFlag" >/dev/null \
        || echo "herdr plugin link failed; run: herdr plugin link $annotateWant" >&2
    fi
  '';
}
