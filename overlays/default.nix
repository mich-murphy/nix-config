# Changes to nixpkgs packages. flake.nix applies `default` to every package set
# it builds: nix-darwin, Home Manager and `packages`. Packages nixpkgs lacks are
# not overlays; they live in pkgs/ and modules take them from `self.packages`.
# DESIGN.md (Patches) says which kind of change goes where.
{ lib }:
rec {
  modifications = import ./modifications;
  security = import ./security.nix;
  default = lib.composeManyExtensions [
    modifications
    security
  ];
}
