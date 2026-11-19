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
  # Annotate's review pane (plannotator-tui) in Tokyo Night Night, from the
  # same Pi theme as modules/home/agents.nix, so the two cannot drift. Each
  # role takes the Pi colour for the same job. Code blocks use the terminal's
  # foreground ("reset"), as Pi's does here, and fenced code is highlighted
  # with Tokyo Night's own TextMate theme. Borders (tables, unfocused note
  # bubbles, the tree divider) take Pi's `dim`: upstream's dark gray shows as
  # terminal_black, too close to the background to make out. Roles Pi has no
  # colour for take Tokyo Night's palette instead of plannotator-tui's xterm
  # greys and tints: the comment and looks-good highlights are the note
  # colours blended into the background, as Tokyo Night makes its diff
  # backgrounds; the toolbar is `fg_gutter` and the cursor, a step firmer,
  # `dark3`. Only the patched build knows code, code-block, link, quote,
  # border and code_theme; upstream's rejects them.
  tokyonight = "${inputs.tokyonight}/extras";
  piColors = (lib.importJSON "${tokyonight}/pi/tokyonight_night.json").colors;
  # The whole palette, which only some of the extras (such as opencode's) list.
  palette = (lib.importJSON "${tokyonight}/opencode/tokyonight_night.json").defs;
  # Tokyo Night's util.blend: `alpha` of `fg` over `bg`, per channel, rounded.
  blend =
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
  annotateConfig = (pkgs.formats.toml { }).generate "plannotator-tui-config.toml" {
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
      comment-bg = blend piColors.warning 0.3 palette.bg;
      looks-good-bg = blend piColors.success 0.3 palette.bg;
      block = piColors.toolPendingBg;
      toolbar = palette.fg_gutter;
      cursor = palette.dark3;
      selection = piColors.selectedBg;
      code = piColors.mdCode;
      code-block = "reset";
      link = piColors.mdLink;
      quote = piColors.mdQuote;
      border = piColors.dim;
    };
  };
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
  xdg.configFile."plannotator-tui/config.toml".source = annotateConfig;

  # Keeps the plugin folder alive for garbage collection; Herdr records the
  # store path itself, not this link.
  xdg.dataFile."herdr-plugins/annotate".source = annotate;

  home.activation.herdrIntegrations = lib.hm.dag.entryAfter predecessors ''
    herdrStatus=$(${herdr} integration status 2>/dev/null || true)
    for target in ${lib.concatStringsSep " " integrations}; do
      if ! grep -q "^$target: current " <<<"$herdrStatus"; then
        run ${herdr} integration install "$target" \
          || echo "herdr integration install $target failed; rerun it by hand" >&2
      fi
    done
  '';

  # Registers the Nix-built Annotate folder with `herdr plugin link`. Herdr
  # stores the resolved store path, so this relinks whenever the folder
  # changes and does nothing otherwise. Linking replaces a GitHub install of
  # the same plugin and keeps its config and saved annotations; it works with
  # no server running. `link` re-enables a plugin, so the current state is
  # passed back (`.enabled != false`, because jq's `//` treats false as
  # missing).
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
