{
  description = "Nix flake to configure personal M2 Macbook Air";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    darwin.url = "github:nix-darwin/nix-darwin/master";
    darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    darwin,
    home-manager,
    ...
  } @ inputs: {
    darwinConfigurations.macbook = darwin.lib.darwinSystem {
      specialArgs = {inherit inputs;};
      modules = [
        ./hosts/laptop
        home-manager.darwinModules.home-manager
        {
          home-manager.useGlobalPkgs = true;
          home-manager.useUserPackages = true;
          home-manager.users.mm = import ./home/home.nix;
          home-manager.backupFileExtension = "backup";
        }
      ];
    };
  };
}
