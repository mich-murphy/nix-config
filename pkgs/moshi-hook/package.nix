# Moshi's prebuilt hook daemon and CLI (nixpkgs has none; its `moshi` is an
# unrelated project). URLs and hashes come from the generated formula at
# https://github.com/rjyo/homebrew-moshi/blob/main/Formula/moshi-hook.rb.
# To update: change `version`, then copy each platform's sha256 from that
# formula and convert it with `nix hash convert --hash-algo sha256 <hex>`.
{
  lib,
  stdenvNoCC,
  fetchurl,
  testers,
  runCommand,
}:
let
  version = "0.4.50";
  sources = {
    aarch64-darwin = {
      url = "https://cdn.getmoshi.app/hook/v${version}/moshi-hook-app_Darwin_arm64.tar.gz";
      hash = "sha256-Ol/Kabv1PcKQQyST+CiNB2vOj7uIk+F0CZSlSpoznt8=";
    };
    x86_64-linux = {
      url = "https://cdn.getmoshi.app/hook/v${version}/moshi-hook_Linux_x86_64.tar.gz";
      hash = "sha256-eTxWsvAsPjQ24vglI7tuFTi//03j7Ji2seknDW8WfaM=";
    };
  };
  system = stdenvNoCC.hostPlatform.system;
in
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "moshi-hook";
  inherit version;

  src = fetchurl (sources.${system} or (throw "moshi-hook: unsupported system ${system}"));
  # The archives keep their files at the root; unpack into a directory of our
  # own so stdenv never has to chmod the build directory itself.
  unpackCmd = ''mkdir source && tar -xzf "$curSrc" -C source'';
  sourceRoot = "source";

  dontConfigure = true;
  dontBuild = true;
  # Keep the notarized macOS bundle and the static Linux binary byte-for-byte
  # as published; fixup would strip or patch them and break the signature.
  dontFixup = true;

  installPhase = ''
    runHook preInstall
  ''
  + (
    if stdenvNoCC.hostPlatform.isDarwin then
      ''
        mkdir -p $out/Applications $out/bin
        cp -R MoshiHook.app $out/Applications/
        for exe in moshi-hook mata; do
          if [ -e $out/Applications/MoshiHook.app/Contents/MacOS/$exe ]; then
            ln -s $out/Applications/MoshiHook.app/Contents/MacOS/$exe $out/bin/$exe
          fi
        done
      ''
    else
      ''
        install -Dm755 moshi-hook $out/bin/moshi-hook
      ''
  )
  + ''
    ln -s moshi-hook $out/bin/moshi
    runHook postInstall
  '';

  # On macOS, running a binary from inside an app bundle makes the system tag
  # the bundle with com.apple.macl, and App Management then stops nix-daemon
  # from deleting it, so garbage collection fails on that store path. Check the
  # bundle's declared version instead of running it.
  passthru.tests.version =
    if stdenvNoCC.hostPlatform.isDarwin then
      runCommand "moshi-hook-${version}-test-version" { } ''
        app=${finalAttrs.finalPackage}/Applications/MoshiHook.app/Contents
        test -x $app/MacOS/moshi-hook
        grep -A1 '<key>CFBundleShortVersionString</key>' $app/Info.plist \
          | grep -qF '<string>${version}</string>'
        touch $out
      ''
    else
      testers.testVersion {
        package = finalAttrs.finalPackage;
        command = "moshi-hook --version";
      };

  meta = {
    description = "Daemon and CLI that bridges coding agents to the Moshi mobile app";
    homepage = "https://getmoshi.app";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = builtins.attrNames sources;
    mainProgram = "moshi-hook";
  };
})
