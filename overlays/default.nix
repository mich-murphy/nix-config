# Every overlay this flake applies. flake.nix applies `default` to each package
# set it builds (nix-darwin, Home Manager and `packages`), so modules use
# `pkgs.<name>` for all of them. DESIGN.md (Patches) says which change goes
# where.
{ lib }:
rec {
  additions = import ./additions.nix;
  modifications = import ./modifications.nix;
  security = import ./security.nix;
  default = lib.composeManyExtensions [
    additions
    modifications
    security
  ];
}
