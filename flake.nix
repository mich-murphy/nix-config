{
  description = "Nix flake to configure personal M2 Macbook Air and homelab";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    darwin.url = "github:lnl7/nix-darwin";
    darwin.inputs.nixpkgs.follows = "nixpkgs";

    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    disko.url = "github:nix-community/disko";
    disko.inputs.nixpkgs.follows = "nixpkgs";

    nur.url = "github:nix-community/NUR";

    agenix.url = "github:ryantm/agenix";
    agenix.inputs.nixpkgs.follows = "nixpkgs";
    agenix.inputs.home-manager.follows = "home-manager";

    deploy-rs.url = "github:serokell/deploy-rs";
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs";

    impermanence.url = "github:nix-community/impermanence";

    # nix-minecraft.url = "github:Infinidoge/nix-minecraft";
  };

  outputs = {
    self,
    nixpkgs,
    darwin,
    home-manager,
    disko,
    nur,
    agenix,
    deploy-rs,
    impermanence,
    # nix-minecraft,
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
        {
          nixpkgs.overlays = [
            nur.overlays.default
          ];
        }
      ];
    };

    nixosConfigurations.media = nixpkgs.lib.nixosSystem {
      specialArgs = {inherit inputs;};
      modules = [
        ./hosts/media
        agenix.nixosModules.default
        {
          nixpkgs.overlays = [
            # nix-minecraft.overlay
          ];
        }
      ];
    };

    nixosConfigurations.services = nixpkgs.lib.nixosSystem {
      specialArgs = {inherit inputs;};
      modules = [
        ./hosts/services
        agenix.nixosModules.default
        disko.nixosModules.disko
      ];
    };

    nixosConfigurations.downloads = nixpkgs.lib.nixosSystem {
      specialArgs = {inherit inputs;};
      modules = [
        ./hosts/downloads
        agenix.nixosModules.default
      ];
    };

    deploy.nodes = {
      media = {
        hostname = "media";
        remoteBuild = true;
        profiles.system = {
          user = "root";
          sshUser = "mm";
          sshOpts = ["-o" "StrictHostKeyChecking=no"];
          path = deploy-rs.lib.x86_64-linux.activate.nixos self.nixosConfigurations.media;
        };
      };
      services = {
        hostname = "services";
        remoteBuild = true;
        profiles.system = {
          user = "root";
          sshUser = "mm";
          sshOpts = ["-o" "StrictHostKeyChecking=no"];
          path = deploy-rs.lib.x86_64-linux.activate.nixos self.nixosConfigurations.services;
        };
      };
      downloads = {
        hostname = "10.77.2.101";
        remoteBuild = true;
        profiles.system = {
          user = "root";
          sshUser = "mm";
          sshOpts = ["-o" "StrictHostKeyChecking=no"];
          path = deploy-rs.lib.x86_64-linux.activate.nixos self.nixosConfigurations.downloads;
        };
      };
    };

    # disabled until remote testing added https://github.com/serokell/deploy-rs/issues/167
    # checks = builtins.mapAttrs (system: deployLib: deployLib.deployChecks self.deploy) deploy-rs.lib;
  };
}
