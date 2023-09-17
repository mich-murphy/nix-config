{
  description = "Nix flake to configure personal M2 Macbook Air and homelab";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "github:nixos/nixpkgs/nixos-23.05";
    nixpkgs-darwin-stable.url = "github:nixos/nixpkgs/nixpkgs-23.05-darwin";

    darwin.url = "github:lnl7/nix-darwin";
    darwin.inputs.nixpkgs.follows = "nixpkgs";

    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    nur.url = "github:nix-community/NUR";

    agenix.url = "github:ryantm/agenix";
    agenix.inputs.nixpkgs.follows = "nixpkgs";

    deploy-rs.url = "github:serokell/deploy-rs";
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
  in
  {
    darwinConfigurations.macbook = darwin.lib.darwinSystem {
      system = "aarch64-darwin";
      specialArgs = { inherit inputs user host; };
      modules = [
        ./hosts/laptop/darwin-configuration.nix
        home-manager.darwinModules.home-manager {
          home-manager.useGlobalPkgs = true;
          home-manager.useUserPackages = true;
          home-manager.extraSpecialArgs = { inherit user; };
          home-manager.users.${user} = import ./hosts/laptop/home.nix;
        }
        {
          nixpkgs.overlays = [
            nur.overlay
          ];
        }
      ];
    };

    nixosConfigurations.media = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [
        ./hosts/media/configuration.nix
        agenix.nixosModules.default
      ];
    };

    deploy.nodes.media = {
      hostname = "media";
      remoteBuild = true;
      profiles.system = {
        user = "root";
        sshUser = "mm";
        sshOpts = [ "-o" "StrictHostKeyChecking=no" ];
        path = deploy-rs.lib.x86_64-linux.activate.nixos self.nixosConfigurations.media;
      };
    };

      # disabled until remote testing added https://github.com/serokell/deploy-rs/issues/167
      # checks = builtins.mapAttrs (system: deployLib: deployLib.deployChecks self.deploy) deploy-rs.lib;
    };
  }
