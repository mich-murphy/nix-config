{
  description = "nix-darwin and Home Manager configuration for my machines";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    darwin.url = "github:nix-darwin/nix-darwin/master";
    darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    hunk.url = "github:modem-dev/hunk";
    hunk.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{
      nixpkgs,
      darwin,
      home-manager,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      systems = [ "aarch64-darwin" ];
      forAllSystems = lib.genAttrs systems;
      packagesFor =
        system:
        import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

      # A Mac: hosts/<host>/default.nix for the system and
      # hosts/<host>/home.nix for its Home Manager additions.
      mkDarwin =
        host:
        darwin.lib.darwinSystem {
          specialArgs = { inherit inputs; };
          modules = [
            home-manager.darwinModules.home-manager
            ./hosts/${host}
            (
              { pkgs, ... }:
              {
                nixpkgs.config.allowUnfree = true;

                # Nix is managed by the Determinate installer.
                nix.enable = false;

                system.primaryUser = "mm";
                users.users.mm = {
                  home = "/Users/mm";
                  shell = pkgs.fish;
                };
                programs.fish.enable = true;

                home-manager = {
                  useGlobalPkgs = true;
                  useUserPackages = true;
                  backupFileExtension = "backup";
                  extraSpecialArgs = { inherit inputs; };
                  users.mm.imports = [
                    ./modules/home
                    ./hosts/${host}/home.nix
                  ];
                };
              }
            )
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

      darwinConfigurations = lib.genAttrs [
        "macbook"
        "laptop"
      ] mkDarwin;

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
        }
        # Build every Mac's system and home: <host>-system, <host>-home.
        // lib.concatMapAttrs (host: darwin: {
          "${host}-system" = darwin.config.system.build.toplevel;
          "${host}-home" = darwin.config.home-manager.users.mm.home.activationPackage;
        }) inputs.self.darwinConfigurations;
    };
}
