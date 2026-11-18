# Herdr's Annotate plugin (plannotator/herdr-annotate) as a finished plugin
# folder, for `herdr plugin link`. Installed from GitHub, its build steps
# download upstream's prebuilt binaries into bin/; here both are built by Nix,
# plannotator-tui with this repo's patches. modules/home/herdr.nix links it.
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

  # The terminal-annotation runtime the plugin's actions and popups run.
  runtime = rustPlatform.buildRustPackage {
    pname = "herdr-annotate";
    version = "0.2.4";
    inherit src;
    sourceRoot = "${src.name}/rust";
    cargoHash = "sha256-zWXvnpw83Uhra0PrZWuPHvM+nYu0V5UhZbtCgL3HjqM=";
    # These open a pseudo-terminal, and the Mac's Linux builder has none
    # (`openpt` fails). GitHub CI's x86_64-linux runner still runs them.
    checkFlags = lib.optionals stdenv.hostPlatform.isLinux [
      "--skip=termination::tests::a_terminal_reads_as_hung_up_only_after_its_other_end_closes"
      "--skip=termination::tests::piped_stdin_is_passed_over_for_the_terminal"
    ];
    meta.mainProgram = "herdr-annotate";
  };
in
stdenvNoCC.mkDerivation {
  pname = "herdr-plugin-annotate";
  version = "0.10.0";
  inherit src;

  dontConfigure = true;
  dontBuild = true;

  # The manifest's actions run ./bin/<name>.exe. Each fetch script also
  # writes a stamp beside its binary and treats a matching one as installed;
  # the stamps are reproduced (herdr-annotate's with a newline,
  # plannotator-tui's without), so running a fetch script by hand changes
  # nothing. The check stops a plugin bump that pins another plannotator-tui.
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
