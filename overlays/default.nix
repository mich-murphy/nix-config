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
