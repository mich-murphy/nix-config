{
  description = "personal nix-darwin flake configuration";

  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/nixpkgs-unstable;
    darwin.url = github:lnl7/nix-darwin;
    darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = github:nix-community/home-manager;
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    nur.url = github:nix-community/NUR;
  };

  outputs = inputs @ { self, nixpkgs, darwin, home-manager, nur, ... }:
    let
      user = "mm";
      host = "macbook";
      system = "aarch64-darwin";
      fullName = "Michael Murphy";
      gitEmail = "mich+git@elmurphy.com";
    in
    {
      darwinConfigurations.macbook = darwin.lib.darwinSystem {
        inherit system;
	specialArgs = { inherit user host fullName gitEmail };
        modules = [
	  ./darwin/configuration.nix
	  home-manager.darwinModules.home-manager {
            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.extraSpecialArgs = { inherit user fullName gitEmail; };
            home-manager.users.${user} = import ./darwin/home.nix;
	  };
        ];
      };
    }
}

