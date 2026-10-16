{
  description = "Nix flake for a MacBook";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    darwin.url = "github:nix-darwin/nix-darwin/master";
    darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    hunk.url = "github:modem-dev/hunk";
    # Private repo, fetched over SSH. Pinned to an immutable release tag; built from source
    # with the toolchain locked in that repo, so it doesn't follow this nixpkgs.
    agent-sessions.url = "git+ssh://git@github.com/mich-murphy/memory?ref=refs/tags/v0.3.0";
  };

  outputs = {
    nixpkgs,
    darwin,
    home-manager,
    hunk,
    agent-sessions,
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
      specialArgs = {inherit hunk agent-sessions;};
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
      git-branch-cleanup = let
        pkgs = packagesFor "aarch64-darwin";
      in
        pkgs.runCommand "git-branch-cleanup" {
          nativeBuildInputs = [pkgs.git pkgs.bash];
          RM_MERGED_ALIAS = darwinConfig.config.home-manager.users.mm.programs.git.settings.alias.rm-merged;
        } ''
          bash ${./tests/git-branch-cleanup.sh}
          touch "$out"
        '';
      macbook-system = darwinConfig.config.system.build.toplevel;
      macbook-home = darwinConfig.config.home-manager.users.mm.home.activationPackage;
    };
  };
}
