{
  description = "nix-darwin and Home Manager configuration for my machines";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    darwin.url = "github:nix-darwin/nix-darwin/master";
    darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    deploy-rs.url = "github:serokell/deploy-rs";
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs";
    # Only default.nix and shell.nix, for non-flake use, import flake-compat.
    deploy-rs.inputs.flake-compat.follows = "";
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
      self,
      nixpkgs,
      darwin,
      home-manager,
      deploy-rs,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = lib.genAttrs systems;
      nixpkgsConfig = {
        config.allowUnfree = true;
        overlays = [ self.overlays.default ];
      };
      pkgsFor = forAllSystems (system: import nixpkgs ({ inherit system; } // nixpkgsConfig));

      revision = self.rev or self.dirtyRev or "unknown";

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
                nixpkgs = { inherit (nixpkgsConfig) config overlays; };

                system.configurationRevision = revision;

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

      mkHome =
        host: system:
        home-manager.lib.homeManagerConfiguration {
          pkgs = pkgsFor.${system};
          extraSpecialArgs = { inherit inputs; };
          modules = [
            ./modules/home
            ./hosts/${host}/home.nix
            # Home Manager has no configurationRevision, so dev-01 records the
            # commit in a file; the Macs have `darwin-version`.
            { xdg.stateFile."nix-config/revision".text = "${revision}\n"; }
          ];
        };

      # deploy-rs's lib for dev-01, with nixpkgs' cached deploy-rs binary
      # instead of the flake's own source build.
      deployPkgs = pkgsFor.x86_64-linux.appendOverlays [
        deploy-rs.overlays.default
        (_: prev: {
          deploy-rs = {
            inherit (pkgsFor.x86_64-linux) deploy-rs;
            inherit (prev.deploy-rs) lib;
          };
        })
      ];

      writableLinksCheck =
        pkgs: host: home:
        let
          inherit (home.nixConfig) checkout skills;
          roots = lib.concatStringsSep " or " (lib.optional (checkout != null) checkout ++ [ skills ]);
        in
        pkgs.runCommand "${host}-links" { } ''
          skills=${lib.escapeShellArg skills}
          checkout=${lib.escapeShellArg (toString checkout)}
          found=0
          while IFS= read -r -d "" link; do
            # mkOutOfStoreSymlink makes home-files -> store symlink -> live path.
            # Ordinary store-backed sources are not writable links and are skipped.
            target=$(readlink "$link")
            [ -L "$target" ] || continue
            found=$((found + 1))
            dest=$(readlink "$target")
            if [[ $dest == "$skills" || $dest == "$skills"/* ]]; then
              continue
            fi
            if [[ -n $checkout && $dest == "$checkout"/* ]]; then
              if [ ! -e "${self}/''${dest#"$checkout"/}" ]; then
                echo "$link -> $dest does not exist in the repository"
                exit 1
              fi
              continue
            fi
            echo "$link -> $dest is not in ${roots}"
            exit 1
          done < <(find ${home.home-files} -type l -print0)
          if [ "$found" -eq 0 ]; then
            echo "no writable links found"
            exit 1
          fi
          echo "$found writable links point into ${roots}"
          touch "$out"
        '';
      hostChecks = {
        aarch64-darwin =
          let
            pkgs = pkgsFor.aarch64-darwin;
            lintFiles = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                (lib.fileset.fileFilter (file: file.hasExt "nix" || file.hasExt "md") ./.)
                ./config/markdownlint-cli2.yaml
              ];
            };
          in
          {
            lint =
              pkgs.runCommand "lint"
                {
                  nativeBuildInputs = [
                    pkgs.markdownlint-cli2
                    pkgs.nixf-diagnose
                    pkgs.nixfmt
                  ];
                }
                ''
                  cd ${lintFiles}
                  files=$(find . -name '*.nix')
                  nixfmt --check $files
                  nixf-diagnose $files
                  markdownlint-cli2 --config config/markdownlint-cli2.yaml $(find . -name '*.md')
                  touch "$out"
                '';
          }
          // lib.concatMapAttrs (
            host: darwin:
            let
              home = darwin.config.home-manager.users.mm;
            in
            {
              "${host}-system" = darwin.config.system.build.toplevel;
              "${host}-home" = home.home.activationPackage;
              "${host}-links" = writableLinksCheck pkgsFor.aarch64-darwin host home;
            }
          ) self.darwinConfigurations;

        x86_64-linux =
          let
            home = self.homeConfigurations."dev@dev-01".config;
          in
          {
            dev-01-home = home.home.activationPackage;
            dev-01-links = writableLinksCheck pkgsFor.x86_64-linux "dev-01" home;
          }
          // deployPkgs.deploy-rs.lib.deployChecks self.deploy;
      };
    in
    {
      overlays.default = final: _prev: {
        moshi-hook = final.callPackage ./pkgs/moshi-hook/package.nix { };
      };

      packages = forAllSystems (system: {
        inherit (pkgsFor.${system}) moshi-hook;
      });

      formatter = forAllSystems (system: pkgsFor.${system}.nixfmt-tree);

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        {
          default = pkgs.mkShellNoCC {
            packages = [
              pkgs.nixf-diagnose
              pkgs.nixfmt
              pkgs.nvd
              pkgs.deploy-rs
            ];
          };
        }
      );

      darwinConfigurations = lib.genAttrs [
        "macbook"
        "laptop"
      ] mkDarwin;

      homeConfigurations."dev@dev-01" = mkHome "dev-01" "x86_64-linux";

      deploy.nodes.dev-01 = {
        hostname = "dev-01";
        sshUser = "dev"; # otherwise the Mac's own user name
        # dev is not a trusted Nix user on dev-01, so a closure built on the Mac
        # is refused as unsigned; building there needs no trust.
        remoteBuild = true;
        profiles.home.path =
          deployPkgs.deploy-rs.lib.activate.home-manager
            self.homeConfigurations."dev@dev-01";
      };

      checks = forAllSystems (
        system:
        {
          moshi-hook-version = pkgsFor.${system}.moshi-hook.tests.version;
        }
        // hostChecks.${system}
      );
    };
}
