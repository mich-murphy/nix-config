# Packages nixpkgs lacks, from pkgs/. Nothing in nixpkgs depends on them, so
# adding them rebuilds nothing else.
final: _prev: {
  moshi-hook = final.callPackage ../pkgs/moshi-hook/package.nix { };
  plannotator-tui = final.callPackage ../pkgs/plannotator-tui/package.nix { };
}
