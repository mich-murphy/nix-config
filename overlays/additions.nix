final: _prev: {
  moshi-hook = final.callPackage ../pkgs/moshi-hook/package.nix { };
  plannotator-tui = final.callPackage ../pkgs/plannotator-tui/package.nix { };
  herdr-plugin-annotate = final.callPackage ../pkgs/herdr-plugin-annotate/package.nix { };
  namespace-devbox = final.callPackage ../pkgs/namespace-devbox/package.nix { };
}
