{
  lib,
  stdenv,
  stdenvNoCC,
  rustPlatform,
  fetchFromGitHub,
  plannotator-tui,
}:
let
  rev = "dd147a9faf5ab7d716a31abca070c80af508ffbc";
  src = fetchFromGitHub {
    owner = "plannotator";
    repo = "herdr-annotate";
    inherit rev;
    hash = "sha256-RccMDiN/NvqfnugQZhu6oSDHLu49REWYNqFg50DGhTs=";
  };

  testsNeedingPseudoTerminal = [
    "termination::tests::a_terminal_reads_as_hung_up_only_after_its_other_end_closes"
    "termination::tests::piped_stdin_is_passed_over_for_the_terminal"
  ];

  runtime = rustPlatform.buildRustPackage {
    pname = "herdr-annotate";
    version = "0.2.4";
    inherit src;
    sourceRoot = "${src.name}/rust";
    cargoHash = "sha256-zWXvnpw83Uhra0PrZWuPHvM+nYu0V5UhZbtCgL3HjqM=";
    checkFlags = lib.optionals stdenv.hostPlatform.isLinux (
      map (test: "--skip=${test}") testsNeedingPseudoTerminal
    );
    meta.mainProgram = "herdr-annotate";
  };
in
stdenvNoCC.mkDerivation {
  pname = "herdr-plugin-annotate";
  version = "0.10.0";
  inherit src;

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall

    pinned=$(tr -d '[:space:]' < plannotator-tui.version)
    if [ "$pinned" != "${plannotator-tui.version}" ]; then
      echo "herdr-annotate pins plannotator-tui $pinned; pkgs/plannotator-tui is ${plannotator-tui.version}" >&2
      exit 1
    fi

    mkdir -p $out/bin
    cp -r herdr-plugin.toml herdr-annotate.version plannotator-tui.version scripts LICENSE $out/
    install -m755 ${lib.getExe runtime} $out/bin/herdr-annotate.exe
    install -m755 ${lib.getExe plannotator-tui} $out/bin/plannotator-tui.exe
    echo "${runtime.version}" > $out/bin/herdr-annotate.version
    printf '%s' "${plannotator-tui.version}" > $out/bin/plannotator-tui.version

    runHook postInstall
  '';

  passthru = { inherit runtime; };

  meta = {
    description = "Herdr's Annotate plugin folder, with binaries built by Nix";
    homepage = "https://github.com/plannotator/herdr-annotate";
    license = lib.licenses.mit;
    platforms = lib.platforms.darwin ++ lib.platforms.linux;
  };
}
