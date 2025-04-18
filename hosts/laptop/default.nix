{inputs, ...}: {
  # nix configuration
  # reference: https://daiderd.com/nix-darwin/manual/index.html#sec-options

  imports = [
    ./apps.nix
    ./system.nix
    ./user.nix
  ];

  # create /etc/zshrc which loads nix-darwin environment
  # required if you want to use darwin default shel (zsh)
  programs.zsh.enable = true;

  nixpkgs = {
    config.allowUnfree = true; # allow unfree packages
    hostPlatform = "aarch64-darwin";
  };

  nix = {
    # disabling nix-darwin management in place of Determinate
    enable = false;
    # NOTE: commented this line due to build failure - registry.nixpkgs defined in multiple locations
    # registry.nixpkgs.flake = inputs.nixpkgs; # system wide flake registry
    # weekly garbage collection to minimise disk usage
    settings = {
      trusted-users = ["@admin" "mm"];
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

  system.stateVersion = 4;
}
