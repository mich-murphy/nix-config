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
    underclass.url = "github:ghuntley/underclass";
    underclass.inputs.nixpkgs.follows = "nixpkgs";
    # devenv only builds underclass's development shell.
    underclass.inputs.devenv.follows = "";
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
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = lib.genAttrs systems;
      # Packages not in nixpkgs, one pkgs/<name>/package.nix each.
      localPackages =
        pkgs:
        lib.packagesFromDirectoryRecursive {
          inherit (pkgs) callPackage;
          directory = ./pkgs;
        };
      packagesFor =
        system:
        import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ inputs.self.overlays.default ];
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
                nixpkgs.overlays = [ inputs.self.overlays.default ];

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

      # A Linux host with standalone Home Manager: hosts/<host>/home.nix.
      mkHome =
        host: system:
        home-manager.lib.homeManagerConfiguration {
          pkgs = packagesFor system;
          extraSpecialArgs = { inherit inputs; };
          modules = [
            ./modules/home
            ./hosts/${host}/home.nix
          ];
        };

      # mkOutOfStoreSymlink makes a store path that is itself a symlink. Each
      # one must point into the checkout or the skills clone (a path literal
      # would leave a read-only store copy). Links into the checkout must name a
      # file that exists in this repo (a typo or moved file would leave a
      # dangling link); the skills repo is not a flake input, so its targets
      # cannot be checked. A wrong root cannot be seen from a build.
      aarch64Pkgs = packagesFor "aarch64-darwin";

      # Every host in the flake, with the paths its home declares.
      fleet =
        pkgs:
        import ./apps/fleet.nix {
          inherit pkgs lib;
          hosts =
            lib.mapAttrsToList (name: darwin: {
              inherit name;
              kind = "darwin";
              config = name;
              inherit (darwin.config.home-manager.users.mm.nixConfig) checkout skills;
            }) inputs.self.darwinConfigurations
            ++ lib.mapAttrsToList (config: home: {
              name = lib.last (lib.splitString "@" config);
              kind = "home";
              inherit config;
              inherit (home.config.nixConfig) checkout skills;
            }) inputs.self.homeConfigurations;
        };

      linksCheck =
        pkgs: host: home:
        pkgs.runCommand "${host}-links" { } ''
          checkout=${lib.escapeShellArg home.nixConfig.checkout}
          skills=${lib.escapeShellArg home.nixConfig.skills}
          found=0
          while IFS= read -r -d "" link; do
            target=$(readlink "$link")
            [ -L "$target" ] || continue
            found=$((found + 1))
            dest=$(readlink "$target")
            case "$dest" in
              "$skills" | "$skills"/*) ;;
              "$checkout"/*)
                if [ ! -e "${inputs.self}/''${dest#"$checkout"/}" ]; then
                  echo "$link -> $dest does not exist in the repository"
                  exit 1
                fi
                ;;
              *)
                echo "$link -> $dest is not in $checkout or $skills"
                exit 1
                ;;
            esac
          done < <(find ${home.home-files} -type l -print0)
          if [ "$found" -eq 0 ]; then
            echo "no writable links found"
            exit 1
          fi
          echo "$found writable links point into $checkout or $skills"
          touch "$out"
        '';
    in
    {
      overlays.default = final: _prev: localPackages final;

      packages = forAllSystems (
        system:
        lib.filterAttrs (_: lib.meta.availableOn { inherit system; }) (localPackages (packagesFor system))
      );

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

      homeConfigurations."dev@dev-01" = mkHome "dev-01" "x86_64-linux";

      # `nix run .#deploy` and `nix run .#status`, run from a Mac.
      apps.aarch64-darwin =
        let
          inherit (fleet aarch64Pkgs) deploy status;
          app = drv: description: {
            type = "app";
            program = lib.getExe drv;
            meta.description = description;
          };
        in
        {
          deploy = app deploy "Fast-forward every host's checkouts and switch to the published commit";
          status = app status "Show each host's revisions, local changes, settings drift and versions";
        };

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
          moshi-hook-version = pkgs.moshi-hook.tests.version;
          # Builds the fleet scripts, which runs ShellCheck over them.
          fleet =
            let
              f = fleet pkgs;
            in
            pkgs.linkFarmFromDrvs "fleet" [
              f.host
              f.deploy
              f.status
            ];
        }
        # Build every Mac's system and home, and check its writable links.
        // lib.concatMapAttrs (
          host: darwin:
          let
            home = darwin.config.home-manager.users.mm;
          in
          {
            "${host}-system" = darwin.config.system.build.toplevel;
            "${host}-home" = home.home.activationPackage;
            "${host}-links" = linksCheck pkgs host home;
          }
        ) inputs.self.darwinConfigurations;

      # dev-01's home and links, plus the Linux moshi-hook. These need a Linux
      # builder: `nix flake check --all-systems`.
      checks.x86_64-linux =
        let
          pkgs = packagesFor "x86_64-linux";
          home = inputs.self.homeConfigurations."dev@dev-01".config;
        in
        {
          moshi-hook-version = pkgs.moshi-hook.tests.version;
          dev-01-home = home.home.activationPackage;
          dev-01-links = linksCheck pkgs "dev-01" home;
        };
    };
}
