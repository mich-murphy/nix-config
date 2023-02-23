{
  description = "Nix flake to configure personal M2 Macbook Air and homelab";

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
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs-stable";

    impermanence.url = github:nix-community/impermanence;

    neovim.url = github:neovim/neovim?dir=contrib;
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
    neovim,
    ... 
  }@inputs:
  let
    user = "mm";
    host = "macbook";
  in
  {
    darwinConfigurations.macbook = darwin.lib.darwinSystem {
      system = "aarch64-darwin";
      specialArgs = { inherit user host; };
      modules = [
        ./hosts/laptop/darwin-configuration.nix
        home-manager.darwinModules.home-manager {
          home-manager.useGlobalPkgs = true;
          home-manager.useUserPackages = true;
          home-manager.extraSpecialArgs = { inherit user; };
          home-manager.users.${user} = import ./hosts/laptop/home.nix;
        }
        {
          nixpkgs.overlays = with inputs; [
            nur.overlay
            neovim.overlay
          ];
        }
      ];
    };

    nixosConfigurations.nix-media = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [
        ./hosts/homelab/configuration.nix
        agenix.nixosModules.default
      ];
    };

    deploy.nodes.homelab = {
      hostname = "nix-media";
      remoteBuild = true;
      profiles.system = {
        user = "root";
        sshUser = "mm";
        path = deploy-rs.lib.x86_64-linux.activate.nixos self.nixosConfigurations.nix-media;
      };
    };

      # disabled until remote testing added https://github.com/serokell/deploy-rs/issues/167
      # checks = builtins.mapAttrs (system: deployLib: deployLib.deployChecks self.deploy) deploy-rs.lib;
    };
  }
