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
    # No follows: Determinate recommends its own nixpkgs for FlakeHub Cache hits.
    determinate.url = "https://flakehub.com/f/DeterminateSystems/determinate/3";
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
            inputs.determinate.darwinModules.default
            ./hosts/${host}
            (
              { pkgs, ... }:
              {
                nixpkgs.config.allowUnfree = true;

                # Shown by `darwin-version --configuration-revision`.
                system.configurationRevision = inputs.self.rev or inputs.self.dirtyRev or null;

                # Determinate installs Nix; its module stops nix-darwin managing it.
                determinateNix.enable = true;
                # The module takes over the installer's nix.custom.conf. Allow
                # replacing only the untouched installer defaults (hashes from
                # nix-darwin's modules/nix); an edited file still aborts.
                environment.etc."nix/nix.custom.conf".knownSha256Hashes = [
                  "6787fade1cf934f82db554e78e1fc788705c2c5257fddf9b59bdd963ca6fec63" # DetSys v0.33.0
                  "3bd68ef979a42070a44f8d82c205cfd8e8cca425d91253ec2c10a88179bb34aa" # DetSys v0.34.0
                ];

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
        # <host>-links: mkOutOfStoreSymlink makes a store path that is itself a
        # symlink. Each one must point into the checkout (a path literal would
        # leave a read-only store copy) at a file that exists in this repo (a
        # typo or moved file would leave a dangling link). A wrong checkout
        # root cannot be seen from a build.
        // lib.concatMapAttrs (
          host: darwin:
          let
            home = darwin.config.home-manager.users.mm;
          in
          {
            "${host}-system" = darwin.config.system.build.toplevel;
            "${host}-home" = home.home.activationPackage;
            "${host}-links" = pkgs.runCommand "${host}-links" { } ''
              checkout=${lib.escapeShellArg home.nixConfig.checkout}
              found=0
              while IFS= read -r -d "" link; do
                target=$(readlink "$link")
                [ -L "$target" ] || continue
                found=$((found + 1))
                dest=$(readlink "$target")
                case "$dest" in
                  "$checkout"/*) ;;
                  *)
                    echo "$link -> $dest is not in $checkout"
                    exit 1
                    ;;
                esac
                if [ ! -e "${inputs.self}/''${dest#"$checkout"/}" ]; then
                  echo "$link -> $dest does not exist in the repository"
                  exit 1
                fi
              done < <(find ${home.home-files} -type l -print0)
              if [ "$found" -eq 0 ]; then
                echo "no writable links found"
                exit 1
              fi
              echo "$found writable links point into $checkout"
              touch "$out"
            '';
          }
        ) inputs.self.darwinConfigurations;
    };
}
