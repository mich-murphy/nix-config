final: prev: {
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
