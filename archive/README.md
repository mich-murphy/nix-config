# Archive

This directory preserves the repository's inactive NixOS/media system for
future reference.

## Archived paths

- `archive/hosts/media/` — former media host configuration
- `archive/nixos/` — NixOS modules used by the archived host
- `archive/secrets/` — `agenix` secret declarations and encrypted secret files
  for the archived host

## Active flake scope

The active flake currently retains only:

- `darwinConfigurations.macbook`

## Notes

- The archived tree is not exposed as an active flake output.
- The original relative layout was preserved so the archived configuration can
  be revived more easily in future.
- Re-enabling a NixOS host will require restoring the necessary flake
  inputs/outputs, such as `impermanence`, `agenix`, and an appropriate
  `nixosConfigurations.<name>` output.

## `flake.nix` changes needed to restore the archived host

If you want to build the archived media host directly from `archive/`, add the
following back to `flake.nix`:

```nix
{
  inputs = {
    impermanence.url = "github:nix-community/impermanence";
    impermanence.inputs.nixpkgs.follows = "nixpkgs";

    agenix.url = "github:ryantm/agenix";
    agenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    nixpkgs,
    darwin,
    home-manager,
    impermanence,
    agenix,
    ...
  } @ inputs: let
    ...
  in {
    nixosConfigurations.media = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = {inherit inputs;};
      modules = [
        agenix.nixosModules.default
        ./archive/hosts/media
      ];
    };
  };
}
```

Why these are needed:

- `archive/hosts/media/system.nix` imports
  `inputs.impermanence.nixosModules.impermanence`
- the archived NixOS modules use `age.secrets`, so the agenix NixOS module must
  be imported

If you move the archived files back out of `archive/`, update
`./archive/hosts/media` in the snippet to the restored path.
