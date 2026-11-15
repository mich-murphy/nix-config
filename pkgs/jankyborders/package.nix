# nixpkgs' JankyBorders with one fix: on displays with rounded corners, macOS
# rounds a window's bottom corners more as they near the display's corners, and
# borders kept drawing the normal radius there. The patch is
# `git format-patch` of the fix/screen-corner-radii branch of a JankyBorders
# checkout. Version bumps come from nixpkgs; if the patch stops applying the
# build fails, and README.md says how to refresh it. Drop the patch once
# upstream has the fix.
{ jankyborders }:
jankyborders.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ [ ./screen-corner-radii.patch ];
})
