{ lib }:
rec {
  additions = import ./additions.nix;
  modifications = import ./modifications.nix;
  default = lib.composeManyExtensions [
    additions
    modifications
  ];
}
