# Moshi's prebuilt hook daemon and CLI (nixpkgs has none; its `moshi` is an
# unrelated project). URLs and hashes come from the generated formula at
# https://github.com/rjyo/homebrew-moshi/blob/main/Formula/moshi-hook.rb.
{
  lib,
  stdenvNoCC,
  fetchurl,
  runCommand,
  testers,
}:
let
  version = "0.4.54";
  sources = {
    aarch64-darwin = {
      url = "https://cdn.getmoshi.app/hook/v${version}/moshi-hook-app_Darwin_arm64.tar.gz";
      hash = "sha256-vELnwNxsE+tBXXPo7FkYObSkQsbKOAYQudA3svjm12Q=";
    };
    x86_64-linux = {
      url = "https://cdn.getmoshi.app/hook/v${version}/moshi-hook_Linux_x86_64.tar.gz";
      hash = "sha256-SxPdAppaMe4r3bl7uVY/KBLuwP8exzDpIp00xQ6PeLk=";
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

  passthru = {
    # On macOS, running a binary inside the store's app bundle makes the system
    # tag it with com.apple.macl, and then nix-daemon cannot delete it. Read the
    # version the bundle declares instead of running it.
    tests.version =
      if stdenvNoCC.hostPlatform.isDarwin then
        runCommand "moshi-hook-${finalAttrs.version}-test-version" { } ''
          contents=${finalAttrs.finalPackage}/Applications/MoshiHook.app/Contents
          test -x $contents/MacOS/moshi-hook
          grep -A1 '<key>CFBundleShortVersionString</key>' $contents/Info.plist \
            | grep -qF '<string>${finalAttrs.version}</string>'
          touch $out
        ''
      else
        testers.testVersion {
          package = finalAttrs.finalPackage;
          command = "moshi-hook --version";
        };
  }
  // lib.optionalAttrs stdenvNoCC.hostPlatform.isDarwin {
    app = runCommand "moshi-hook-app" { } ''
      mkdir -p $out/Applications
      ln -s ${finalAttrs.finalPackage}/Applications/MoshiHook.app $out/Applications/
    '';
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
