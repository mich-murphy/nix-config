# Local fixes to nixpkgs packages that are not security fixes. Each one patches
# a package nothing else depends on, so only that package rebuilds.
final: prev: {
  jankyborders = final.callPackage ./jankyborders/package.nix {
    inherit (prev) jankyborders;
  };
}
