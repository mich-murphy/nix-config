# Local fixes to nixpkgs packages that are not security fixes. Each patches a
# package nothing else depends on, so only that package rebuilds. Each asserts
# the upstream version it was written for: a nixpkgs bump stops evaluation, and
# the new release is checked for an upstream fix before the patch carries on.
# Patch files live in patches/<package>/.
final: prev: {
  # On displays with rounded corners, macOS rounds a window's bottom corners
  # more as they near the display's corners, and borders kept drawing the
  # normal radius there. The patch is `git format-patch` of one commit on
  # upstream's v1.9.0 tag; it is not submitted upstream. README.md (Patched
  # packages) says how to refresh it.
  jankyborders =
    assert final.lib.assertMsg (prev.jankyborders.version == "1.9.0") ''
      overlays/modifications.nix: nixpkgs now has JankyBorders ${prev.jankyborders.version},
      and the patch was written for 1.9.0. If the new release fixes the bottom
      corners near rounded display corners, delete the jankyborders entry and
      overlays/patches/jankyborders. Otherwise refresh the patch and update the
      version here (README.md, Patched packages).
    '';
    prev.jankyborders.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./patches/jankyborders/screen-corner-radii.patch ];
    });
}
