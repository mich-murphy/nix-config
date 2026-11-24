{
  lib,
  stdenvNoCC,
  fetchurl,
  testers,
}:
let
  version = "0.0.214";
  sources = {
    aarch64-darwin = {
      url = "https://github.com/namespacelabs/devbox/releases/download/v${version}/devbox_${version}_darwin_arm64.tar.gz";
      hash = "sha256-NI0aujGW7DmfhGvnblKTQJ4TL/2ffortOeqVjYyHjvc=";
    };
    x86_64-linux = {
      url = "https://github.com/namespacelabs/devbox/releases/download/v${version}/devbox_${version}_linux_amd64.tar.gz";
      hash = "sha256-qlZnRI8dsGhwUhqLwAl9NhQilcZy74eQFon9/HmNRLk=";
    };
    aarch64-linux = {
      url = "https://github.com/namespacelabs/devbox/releases/download/v${version}/devbox_${version}_linux_arm64.tar.gz";
      hash = "sha256-eZEXD/1/1dQzTz50QzSiCMwCGUXdaMefmjPMYKaY5dE=";
    };
  };
  system = stdenvNoCC.hostPlatform.system;
in
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "namespace-devbox";
  inherit version;

  src = fetchurl (sources.${system} or (throw "namespace-devbox: unsupported system ${system}"));
  sourceRoot = ".";

  dontConfigure = true;
  dontBuild = true;
  dontFixup = true;

  installPhase = ''
    runHook preInstall
    install -Dm755 devbox $out/bin/devbox
    runHook postInstall
  '';
  passthru.tests.version = testers.testVersion {
    package = finalAttrs.finalPackage;
    command = "HOME=$TMPDIR devbox version";
  };

  meta = {
    description = "Namespace's CLI for managing Devboxes";
    homepage = "https://namespace.so/docs/devbox";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = builtins.attrNames sources;
    mainProgram = "devbox";
  };
})
