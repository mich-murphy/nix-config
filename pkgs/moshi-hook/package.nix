{
  lib,
  stdenvNoCC,
  fetchurl,
  runCommand,
  testers,
}:
let
  version = "0.4.62";
  sources = {
    aarch64-darwin = {
      url = "https://cdn.getmoshi.app/hook/v${version}/moshi-hook-app_Darwin_arm64.tar.gz";
      hash = "sha256-fpIh/kjmOT8LiQQFMgFlLJhUZqqhCjEF78ptX5X+oCM=";
    };
    x86_64-linux = {
      url = "https://cdn.getmoshi.app/hook/v${version}/moshi-hook_Linux_x86_64.tar.gz";
      hash = "sha256-BugdkSdcXMIVzanbdveLPDE8QNhmnpEKEWmP6p2EHNg=";
    };
  };
  system = stdenvNoCC.hostPlatform.system;
in
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "moshi-hook";
  inherit version;

  src = fetchurl (sources.${system} or (throw "moshi-hook: unsupported system ${system}"));
  sourceRoot = ".";

  dontConfigure = true;
  dontBuild = true;
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
