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
let
  testsLinkingFakeHerdrThroughNonAsciiTempDir = [
    "a_failed_read_reports_herdrs_error_without_opening_a_review"
    "a_pane_with_nothing_on_it_fails_without_opening_a_review"
    "an_action_without_a_focused_pane_reads_nothing"
    "print_shows_the_focused_panes_cleaned_output_as_one_code_block"
    "the_review_pane_opens_on_the_output_and_sends_to_the_agent_that_produced_it"
  ];
  testsNeedingPseudoTerminalOrArgumentLimit = [
    "hangup::tests::a_terminal_reads_as_hung_up_only_after_its_other_end_closes"
    "hangup::tests::piped_stdin_is_passed_over_for_the_terminal"
    "hc::agent::tests::a_send_reaches_only_a_ready_agent_and_every_refusal_lets_the_review_copy_or_retry"
    "hc::command::tests::text_over_the_argument_limit_is_a_size_problem_not_a_missing_terminal"
  ];
  skip = map (test: "--skip=${test}");
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "plannotator-tui";
  version = "0.10.0";

  src = fetchFromGitHub {
    owner = "plannotator";
    repo = "plannotator-tui";
    tag = "v${finalAttrs.version}";
    hash = "sha256-I5Gc/YdpybsWvgYNELKLSRGszFJBiyg4C/MKAAQ8CPo=";
  };

  cargoPatches = [
    ../../overlays/patches/plannotator-tui/colour-roles.patch
    ../../overlays/patches/plannotator-tui/code-highlighting.patch
    ../../overlays/patches/plannotator-tui/mermaid-diagrams.patch
    ../../overlays/patches/plannotator-tui/border-role.patch
  ];

  patches = [ ../../overlays/patches/plannotator-tui/test-races.patch ];

  cargoHash = "sha256-HnLPY+d461tdm3pwYhgYx9AUUE+sKsVarLry6bYi/Ss=";

  cargoBuildFlags = [
    "-p"
    "plannotator-tui"
  ];
  cargoTestFlags = [
    "-p"
    "plannotator-tui"
  ];

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ oniguruma ];

  preBuild = ''
    export LIBGIT2_NO_VENDOR=0
    unset LIBSQLITE3_SYS_USE_PKG_CONFIG
  '';

  nativeCheckInputs = [ git ];
  preCheck = ''
    export HOME=$TMPDIR
  '';
  checkFlags =
    skip testsLinkingFakeHerdrThroughNonAsciiTempDir
    ++ lib.optionals stdenv.hostPlatform.isLinux (skip testsNeedingPseudoTerminalOrArgumentLimit);

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
