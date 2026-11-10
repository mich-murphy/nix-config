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
    # Only default.nix, for non-flake use, imports flake-compat.
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
      # Packages not in nixpkgs, one pkgs/<name>/package.nix each.
      localPackages =
        pkgs:
        lib.packagesFromDirectoryRecursive {
          inherit (pkgs) callPackage;
          directory = ./pkgs;
        };
      # The one nixpkgs configuration, shared by the standalone imports below
      # and the nix-darwin hosts.
      nixpkgsConfig = {
        config.allowUnfree = true;
        overlays = [ self.overlays.default ];
      };
      pkgsFor = forAllSystems (system: import nixpkgs ({ inherit system; } // nixpkgsConfig));

      revision = self.rev or self.dirtyRev or "unknown";

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
                nixpkgs = { inherit (nixpkgsConfig) config overlays; };

                # Shown by `darwin-version --configuration-revision`.
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

      # A Linux host with standalone Home Manager: hosts/<host>/home.nix.
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

      # mkOutOfStoreSymlink makes a store path that is itself a symlink. Each
      # one must point into the checkout or the skills clone (a path literal
      # would leave a read-only store copy). Links into the checkout must name a
      # file that exists in this repo (a typo or moved file would leave a
      # dangling link); the skills repo is not a flake input, so its targets
      # cannot be checked. A wrong root cannot be seen from a build. A host
      # without a checkout may link only into the skills clone.
      linksCheck =
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
      # Per-system checks beyond the shared ones. dev-01's need a Linux builder:
      # `nix flake check --all-systems`.
      hostChecks = {
        aarch64-darwin =
          let
            pkgs = pkgsFor.aarch64-darwin;
            # Only what lint reads, so editing app config does not rerun it.
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
          # Build every Mac's system and home, and check its writable links.
          // lib.concatMapAttrs (
            host: darwin:
            let
              home = darwin.config.home-manager.users.mm;
            in
            {
              "${host}-system" = darwin.config.system.build.toplevel;
              "${host}-home" = home.home.activationPackage;
              "${host}-links" = linksCheck pkgsFor.aarch64-darwin host home;
            }
          ) self.darwinConfigurations;

        x86_64-linux =
          let
            home = self.homeConfigurations."dev@dev-01".config;
          in
          {
            dev-01-home = home.home.activationPackage;
            dev-01-links = linksCheck pkgsFor.x86_64-linux "dev-01" home;
          };
      };
    in
    {
      overlays.default = final: _prev: localPackages final;

      packages = forAllSystems (
        system:
        lib.filterAttrs (_: lib.meta.availableOn { inherit system; }) (localPackages pkgsFor.${system})
      );

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

      # `deploy .#dev-01`, run from a Mac. The Mac itself is not a node; it
      # switches with `sudo darwin-rebuild switch --flake .`.
      deploy.nodes.dev-01 = {
        hostname = "dev-01"; # the SSH alias in modules/home/mac/ssh.nix
        sshUser = "dev"; # otherwise the Mac's own user name
        profiles.home.path =
          deployPkgs.deploy-rs.lib.activate.home-manager
            self.homeConfigurations."dev@dev-01";
      };

      checks = forAllSystems (
        system:
        {
          moshi-hook-version = pkgsFor.${system}.moshi-hook.tests.version;
        }
        // deploy-rs.lib.${system}.deployChecks self.deploy
        // hostChecks.${system}
      );
    };
}
