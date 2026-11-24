{
  lib,
  stdenvNoCC,
  fetchurl,
}:
let
  version = "0.5.0";
  sources = {
    aarch64-darwin = {
      url = "https://public.namespaceblobs.com/nixcache/releases/v${version}/nixcache_${version}_darwin_arm64.tar.gz";
      hash = "sha256-A/UEwCi4f3cU5JGy2haGPmpaMQGZmexvDHtUXyP/HVw=";
    };
    x86_64-linux = {
      url = "https://public.namespaceblobs.com/nixcache/releases/v${version}/nixcache_${version}_linux_amd64.tar.gz";
      hash = "sha256-j+C7yneYFfzRj9n4447bFQ834cQkg8IXmgGVotjohKY=";
    };
    aarch64-linux = {
      url = "https://public.namespaceblobs.com/nixcache/releases/v${version}/nixcache_${version}_linux_arm64.tar.gz";
      hash = "sha256-RVRhdMdjQvKEZEgytlF/AX6hdYVnIhTh5ky/fANtC5U=";
    };
  };
  system = stdenvNoCC.hostPlatform.system;
in
stdenvNoCC.mkDerivation {
  pname = "nixcache";
  inherit version;

  src = fetchurl (sources.${system} or (throw "nixcache: unsupported system ${system}"));
  unpackCmd = ''mkdir source && tar -xzf "$curSrc" -C source'';
  sourceRoot = "source";

  dontConfigure = true;
  dontBuild = true;
  dontFixup = true;

  installPhase = ''
    runHook preInstall
    install -Dm755 nixcache $out/bin/nixcache
    runHook postInstall
  '';

  meta = {
    description = "Namespace's local Nix binary cache proxy";
    homepage = "https://namespace.so/docs/solutions/github-actions/caching/nix";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = builtins.attrNames sources;
    mainProgram = "nixcache";
  };
}
