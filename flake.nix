{
  description = "personal nix-darwin flake configuration";

  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/nixos-unstable;
    nixpkgs-stable.url = github:nixos/nixpkgs/nixos-22.11;
    nixpkgs-darwin-stable.url = github:nixos/nixpkgs/nixpkgs-22.11-darwin;

    darwin.url = github:lnl7/nix-darwin;
    darwin.inputs.nixpkgs.follows = "nixpkgs-darwin-stable";

    home-manager.url = github:nix-community/home-manager;
    home-manager.inputs.nixpkgs.follows = "nixpkgs-darwin-stable";

    nur.url = github:nix-community/NUR;

    agenix.url = github:ryantm/agenix;

    deploy-rs.url = github:serokell/deploy-rs;
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs";

    impermanence.url = "github:nix-community/impermanence";
  };

  outputs = { 
    self, 
    nixpkgs, 
    nixpkgs-stable, 
    nixpkgs-darwin-stable, 
    darwin, 
    home-manager, 
    nur, 
    agenix, 
    deploy-rs,
    impermanence,
    ... 
  }@inputs:
    let
      user = "mm";
      host = "macbook";
      gitUser = "mich-murphy";
      gitEmail = "github@elmurphy.com";
    in
    {
      darwinConfigurations.macbook = darwin.lib.darwinSystem {
        system = "aarch64-darwin";
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

      nixosConfigurations.nix-media = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs; };
        modules = [
          ./hosts/homelab/configuration.nix
          agenix.nixosModule
        ];
      };

      deploy.nodes.homelab = {
        hostname = "nix-media";
        remoteBuild = true;
        profiles.system = {
          user = "mm";
          sshUser = "mm";
          path = deploy-rs.lib.x86_64-linux.activate.nixos self.nixosConfigurations.nix-media;
        };
      };

      # disabled until remote testing added https://github.com/serokell/deploy-rs/issues/167
      # checks = builtins.mapAttrs (system: deployLib: deployLib.deployChecks self.deploy) deploy-rs.lib;
    };
}
