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

  outputs = { self, nixpkgs, darwin, home-manager, nur, ... }@inputs:
    let
      user = "mm";
      host = "macbook";
      system = "aarch64-darwin";
      gitUser = "mich-murphy";
      gitEmail = "github@elmurphy.com";
    in
    {
      darwinConfigurations.macbook = darwin.lib.darwinSystem {
        inherit system;
	specialArgs = { inherit user host gitUser gitEmail; };
        modules = [
	  ./hosts/laptop/darwin-configuration.nix
	  home-manager.darwinModules.home-manager {
            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.extraSpecialArgs = { inherit user gitUser gitEmail; };
            home-manager.users.${user} = import ./hosts/laptop/home.nix;
	  }
	  {
            nixpkgs.overlays = with inputs; [
              nur.overlay
            ];
	  }
        ];
      };
    };
}
