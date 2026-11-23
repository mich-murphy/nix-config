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
    deploy-rs.inputs.flake-compat.follows = "";
    determinate.url = "https://flakehub.com/f/DeterminateSystems/determinate/3";
    llm-agents.url = "github:numtide/llm-agents.nix";
    tokyonight.url = "github:folke/tokyonight.nvim";
    tokyonight.flake = false;
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
              { config, ... }:
              {
                nixpkgs = { inherit (nixpkgsConfig) config overlays; };

                system.configurationRevision = revision;

                home-manager = {
                  extraSpecialArgs = { inherit inputs; };
                  users.${config.system.primaryUser}.imports = [
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
            { xdg.stateFile."nix-config/revision".text = "${revision}\n"; }
          ];
        };

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
            store_link=$(readlink "$link")
            [ -L "$store_link" ] || continue
            found=$((found + 1))
            dest=$(readlink "$store_link")
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
        aarch64-darwin = lib.concatMapAttrs (
          host: darwin:
          let
            home = darwin.config.home-manager.users.${darwin.config.system.primaryUser};
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
      overlays = import ./overlays { inherit lib; };

      packages = forAllSystems (
        system:
        {
          inherit (pkgsFor.${system}) moshi-hook plannotator-tui herdr-plugin-annotate;
        }
        // lib.optionalAttrs pkgsFor.${system}.stdenv.hostPlatform.isDarwin {
          inherit (pkgsFor.${system}) jankyborders;
        }
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

      darwinConfigurations = lib.genAttrs [ "laptop" ] mkDarwin;

      homeConfigurations."dev@dev-01" = mkHome "dev-01" "x86_64-linux";

      deploy.nodes.dev-01 = {
        hostname = "dev-01";
        sshUser = "dev";
        remoteBuild = true;
        profiles.home.path =
          deployPkgs.deploy-rs.lib.activate.home-manager
            self.homeConfigurations."dev@dev-01";
      };

      checks = forAllSystems (
        system:
        {
          moshi-hook-version = pkgsFor.${system}.moshi-hook.tests.version;
          plannotator-tui-version = pkgsFor.${system}.plannotator-tui.tests.version;
        }
        // hostChecks.${system}
      );
    };
}
