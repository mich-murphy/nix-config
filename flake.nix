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
    nixpkgs,
    darwin,
    home-manager,
    ...
  } @ inputs: let
    system = "aarch64-darwin";
    pkgs = nixpkgs.legacyPackages.${system};
    darwinConfig = darwin.lib.darwinSystem {
      specialArgs = {inherit inputs;};
      modules = [
        {
          nixpkgs.overlays = [
            (final: prev: {
              fakepkg = name: final.runCommand name {} "mkdir $out";
            })
          ];
        }
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
  in {
    formatter.${system} = pkgs.writeShellApplication {
      name = "nix-config-fmt";
      runtimeInputs = [pkgs.alejandra];
      text = ''
        exec alejandra "''${@:-.}"
      '';
    };

    darwinConfigurations.macbook = darwinConfig;

    checks.${system} = {
      macbook-system = darwinConfig.config.system.build.toplevel;
      macbook-home = darwinConfig.config.home-manager.users.mm.home.activationPackage;
    };
  };
}
