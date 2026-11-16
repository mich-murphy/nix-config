# nixpkgs' JankyBorders with one fix: on displays with rounded corners, macOS
# rounds a window's bottom corners more as they near the display's corners, and
# borders kept drawing the normal radius there. The patch is
# `git format-patch` of the fix/screen-corner-radii branch of a JankyBorders
# checkout. It is not submitted upstream.
#
# Version bumps come from nixpkgs. The assertion stops evaluation when nixpkgs
# moves past the version the patch was written for, so a new release is checked
# for an upstream fix before the patch carries on. README.md (Patched packages)
# says how to refresh it.
{ lib, jankyborders }:
assert lib.assertMsg (jankyborders.version == "1.9.0") ''
  overlays/modifications/jankyborders: nixpkgs now has JankyBorders ${jankyborders.version},
  and the patch was written for 1.9.0. If the new release fixes the bottom
  corners near rounded display corners, delete this directory and its entry in
  overlays/modifications/default.nix. Otherwise refresh the patch and update
  the version here (README.md, Patched packages).
'';
jankyborders.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ [ ./screen-corner-radii.patch ];
})
