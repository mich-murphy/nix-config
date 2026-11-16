# Local fixes to nixpkgs packages that are not security fixes. Each patches a
# package nothing else depends on, so only that package rebuilds. Each asserts
# the upstream version it was written for: a nixpkgs bump stops evaluation, and
# the new release is checked for an upstream fix before the patch carries on.
# Patch files live in patches/<package>/.
final: prev: {
  # Three fixes, each `git format-patch` of one commit, applied in order on
  # upstream's v1.9.0 tag; none is submitted upstream. README.md (Patched
  # packages) says how to refresh them.
  # - screen-corner-radii: on displays with rounded corners, macOS rounds a
  #   window's bottom corners more as they near the display's corners, and
  #   borders kept drawing the normal radius there.
  # - mission-control: on macOS 27, borders stayed on screen at full size over
  #   Mission Control (upstream issue 209). Based on
  #   andrewcincotta/JankyBorders@0017b53.
  # - attached-sheets: a border ordered above its window also covered the
  #   window's sheets and open panels.
  jankyborders =
    assert final.lib.assertMsg (prev.jankyborders.version == "1.9.0") ''
      overlays/modifications.nix: nixpkgs now has JankyBorders ${prev.jankyborders.version},
      and the patches were written for 1.9.0. Drop each patch the new release
      fixes (bottom corners near rounded display corners, borders over Mission
      Control, borders over sheets), refresh the rest and update the version
      here (README.md, Patched packages).
    '';
    prev.jankyborders.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [
        ./patches/jankyborders/screen-corner-radii.patch
        ./patches/jankyborders/mission-control.patch
        ./patches/jankyborders/attached-sheets.patch
      ];
    });
}
