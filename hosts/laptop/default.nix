{
  pkgs,
  inputs,
  ...
}: {
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

  services.nix-daemon.enable = true; # auto upgrade nix package and daemon service

  nixpkgs = {
    config.allowUnfree = true; # allow unfree packages
    hostPlatform = "aarch64-darwin";
  };

  nix = {
    package = pkgs.nixUnstable; # nix package instance to use in system
    registry.nixpkgs.flake = inputs.nixpkgs; # system wide flake registry
    # weekly garbage collection to minimise disk usage
    gc = {
      automatic = true;
      interval.Day = 7;
      options = "--delete-older-than 7d";
    };
    settings = {
      trusted-users = ["@admin" "mm"];
      auto-optimise-store = true;
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
