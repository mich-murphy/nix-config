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

  outputs =
    {
      nixpkgs,
      darwin,
      home-manager,
      hunk,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      systems = [ "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      packagesFor =
        system:
        import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
      darwinConfig = darwin.lib.darwinSystem {
        specialArgs = { inherit hunk; };
        modules = [
          home-manager.darwinModules.home-manager
          ./configuration.nix
        ];
      };
    in
    {
      formatter = forAllSystems (system: (packagesFor system).nixfmt-tree);

      devShells = forAllSystems (
        system:
        let
          pkgs = packagesFor system;
        in
        {
          default = pkgs.mkShellNoCC {
            packages = [
              pkgs.nixf-diagnose
              pkgs.nixfmt
              pkgs.nvd
            ];
          };
        }
      );

      darwinConfigurations.macbook = darwinConfig;

      checks.aarch64-darwin =
        let
          pkgs = packagesFor "aarch64-darwin";
          # Only .nix files, so editing docs or app config does not rerun lint.
          nixFiles = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.fileFilter (file: file.hasExt "nix") ./.;
          };
        in
        {
          lint =
            pkgs.runCommand "lint"
              {
                nativeBuildInputs = [
                  pkgs.nixf-diagnose
                  pkgs.nixfmt
                ];
              }
              ''
                cd ${nixFiles}
                files=$(find . -name '*.nix')
                nixfmt --check $files
                nixf-diagnose $files
                touch "$out"
              '';
          macbook-system = darwinConfig.config.system.build.toplevel;
          macbook-home = darwinConfig.config.home-manager.users.mm.home.activationPackage;
        };
    };
}
