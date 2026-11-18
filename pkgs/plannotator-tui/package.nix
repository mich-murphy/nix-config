# plannotator-tui, the terminal Markdown annotator behind Herdr's Annotate
# plugin, built from source (nixpkgs has none), with local patches that are
# not submitted upstream. README.md (Patched packages) says how to refresh them.
{
  lib,
  stdenv,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  oniguruma,
  git,
  testers,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "plannotator-tui";
  version = "0.10.0";

  src = fetchFromGitHub {
    owner = "plannotator";
    repo = "plannotator-tui";
    tag = "v${finalAttrs.version}";
    hash = "sha256-I5Gc/YdpybsWvgYNELKLSRGszFJBiyg4C/MKAAQ8CPo=";
  };

  # Each patch is `git format-patch` of one commit, applied in order on the
  # v0.10.0 tag. They are cargoPatches, not patches, because the second
  # changes Cargo.lock and the vendored dependencies must follow it.
  # - colour-roles: [colors] roles for inline code, code blocks, links and
  #   quotes, which upstream hardcodes.
  # - code-highlighting: fenced code highlighted with the TextMate theme
  #   [ui] code_theme names; unset, nothing is highlighted, as upstream.
  # - mermaid-diagrams: fenced mermaid blocks drawn as text by boxmaid,
  #   [ui] mermaid; a diagram it cannot draw shows its source.
  cargoPatches = [
    ../../overlays/patches/plannotator-tui/colour-roles.patch
    ../../overlays/patches/plannotator-tui/code-highlighting.patch
    ../../overlays/patches/plannotator-tui/mermaid-diagrams.patch
  ];

  cargoHash = "sha256-HnLPY+d461tdm3pwYhgYx9AUUE+sKsVarLry6bYi/Ss=";

  # The repository is a workspace; build and test only the app.
  cargoBuildFlags = [
    "-p"
    "plannotator-tui"
  ];
  cargoTestFlags = [
    "-p"
    "plannotator-tui"
  ];

  # nixpkgs' cargo setup exports LIBGIT2_NO_VENDOR=1 before configure, and
  # diffs-rs's build script refuses a system libgit2: its Git patch parser is
  # checked only against the copy git2's vendored-libgit2 feature builds.
  # It also exports LIBSQLITE3_SYS_USE_PKG_CONFIG=1, which overrides rusqlite's
  # `bundled` feature and links a system SQLite; unset it to build the SQLite
  # upstream bundles, as its release does. Both C libraries are compiled from
  # the vendored crates, so no native inputs are needed.
  # Highlighting uses syntect's oniguruma regex engine. nixpkgs' cargo setup
  # exports RUSTONIG_SYSTEM_LIBONIG=1, so onig_sys links nixpkgs' oniguruma,
  # as bat does, rather than compiling its own copy.
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ oniguruma ];

  preBuild = ''
    export LIBGIT2_NO_VENDOR=0
    unset LIBSQLITE3_SYS_USE_PKG_CONFIG
  '';

  # The integration tests run `git` to make repositories, compile a fake
  # Herdr with `rustc` (already on PATH) and write under HOME.
  nativeCheckInputs = [ git ];
  preCheck = ''
    export HOME=$TMPDIR
  '';
  # These tests link their fake Herdr with `rustc` into a temporary directory
  # whose name contains "ü", and nixpkgs' cc-wrapper garbles that non-ASCII
  # path, so the linker cannot find its objects. Not a code defect; upstream's
  # CI runs them.
  checkFlags = [
    "--skip=a_failed_read_reports_herdrs_error_without_opening_a_review"
    "--skip=a_pane_with_nothing_on_it_fails_without_opening_a_review"
    "--skip=an_action_without_a_focused_pane_reads_nothing"
    "--skip=print_shows_the_focused_panes_cleaned_output_as_one_code_block"
    "--skip=the_review_pane_opens_on_the_output_and_sends_to_the_agent_that_produced_it"
  ]
  # On the Mac's Linux builder: the hangup tests need a pseudo-terminal and the
  # sandbox has none (`openpt` fails), and the hc tests expect the exec of a
  # 4 MiB argument to be refused, which that emulated x86_64 builder allows.
  # GitHub CI's real x86_64-linux runner is the authoritative Linux check.
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    "--skip=hangup::tests::a_terminal_reads_as_hung_up_only_after_its_other_end_closes"
    "--skip=hangup::tests::piped_stdin_is_passed_over_for_the_terminal"
    "--skip=hc::agent::tests::a_send_reaches_only_a_ready_agent_and_every_refusal_lets_the_review_copy_or_retry"
    "--skip=hc::command::tests::text_over_the_argument_limit_is_a_size_problem_not_a_missing_terminal"
  ];

  passthru.tests.version = testers.testVersion { package = finalAttrs.finalPackage; };

  meta = {
    description = "Annotate Markdown in the terminal, standalone and as a Herdr plugin";
    homepage = "https://github.com/plannotator/plannotator-tui";
    changelog = "https://github.com/plannotator/plannotator-tui/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mit;
    platforms = lib.platforms.darwin ++ lib.platforms.linux;
    mainProgram = "plannotator-tui";
  };
})
