self: super:

rec {
  ranger = self.callPackage ./pkgs/ranger.nix { };
}
