{
  inputs,
  ...
}: {
  # nix configuration
  # reference: https://nixos.org/manual/nix/stable/command-ref/conf-file.html#name

  imports = [
    ./apps.nix
    ./disks.nix
    ./security.nix
    ./system.nix
    ./user.nix
  ];

  nixpkgs = {
    config.allowUnfree = true; # allow unfree packages
    hostPlatform = "x86_64-linux";
  };

  nix = {
    registry.nixpkgs.flake = inputs.nixpkgs; # system wide flake registry
    # weekly garbage collection to minimise disk usage
    gc = {
      automatic = true;
      options = "--delete-older-than 7d";
    };
    settings = {
      auto-optimise-store = true;
      allowed-users = ["@wheel"];
      builders-use-substitutes = true; # allow remote builders to use their own cache
      # set additional cache with keys
      substituters = [
        "https://nix-community.cachix.org"
      ];
      trusted-public-keys = [
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      ];
      warn-dirty = false;
    };
    # enable flakes globally
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  system.stateVersion = "23.11";
}
