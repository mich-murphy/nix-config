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
  }: let
    system = "aarch64-darwin";
    pkgs = nixpkgs.legacyPackages.${system};
    darwinConfig = darwin.lib.darwinSystem {
      modules = [
        home-manager.darwinModules.home-manager
        ./configuration.nix
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
