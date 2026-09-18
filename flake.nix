{
  description = "Nix flake for a MacBook";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    darwin.url = "github:nix-darwin/nix-darwin/master";
    darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    hunk.url = "github:modem-dev/hunk";
  };

  outputs = {
    nixpkgs,
    darwin,
    home-manager,
    hunk,
    ...
  }: let
    systems = ["aarch64-darwin"];
    forAllSystems = nixpkgs.lib.genAttrs systems;
    packagesFor = system:
      import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };
    darwinConfig = darwin.lib.darwinSystem {
      specialArgs = {inherit hunk;};
      modules = [
        home-manager.darwinModules.home-manager
        ./configuration.nix
      ];
    };
  in {
    formatter = forAllSystems (system: let
      pkgs = packagesFor system;
    in
      pkgs.writeShellApplication {
        name = "nix-config-fmt";
        runtimeInputs = [pkgs.alejandra];
        text = ''
          exec alejandra "''${@:-.}"
        '';
      });

    darwinConfigurations.macbook = darwinConfig;

    checks.aarch64-darwin = {
      macbook-system = darwinConfig.config.system.build.toplevel;
      macbook-home = darwinConfig.config.home-manager.users.mm.home.activationPackage;
      hunk = hunk.packages.aarch64-darwin.hunk;
      opencode = (packagesFor "aarch64-darwin").opencode;
    };
  };
}
