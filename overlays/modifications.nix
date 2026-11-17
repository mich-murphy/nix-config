# Local fixes to nixpkgs packages that are not security fixes: patches, and a
# newer release where nixpkgs is more than three releases behind upstream.
# Each changes a package nothing else depends on, so only that package
# rebuilds. Each asserts the nixpkgs version it was written for: a nixpkgs bump
# stops evaluation, and the entry is checked, refreshed or deleted before it
# carries on. Patch files live in patches/<package>/.
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

  # tw93's Mole. nixpkgs had 1.58.0 against upstream's 1.72.0: its update bot
  # builds on x86_64-linux and this package is Darwin-only, so every automatic
  # bump fails. Delete this entry once nixpkgs reaches 1.72.0.
  mole-cleaner =
    assert final.lib.assertMsg (prev.mole-cleaner.version == "1.58.0") ''
      overlays/modifications.nix: nixpkgs now has mole-cleaner ${prev.mole-cleaner.version}.
      Delete the entry if that is 1.72.0 or later; otherwise update the version
      asserted here.
    '';
    prev.mole-cleaner.overrideAttrs (
      finalAttrs: _: {
        version = "1.72.0";
        src = final.fetchFromGitHub {
          owner = "tw93";
          repo = "Mole";
          tag = "V${finalAttrs.version}";
          hash = "sha256-pNRp7GnjfqEWyiEmHqHagzJagzZDpU/6oeLHc6wWiB8=";
        };
        vendorHash = "sha256-OQ7QTSuulopAS+2Jet2roZVGPMXgJUav73du+MJlt6o=";
      }
    );
}
