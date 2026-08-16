{
  description = "Nix flake for a MacBook and the ai-dev Linux home";
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
    systems = [
      "aarch64-darwin"
      "x86_64-linux"
    ];
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
    aiDev = home-manager.lib.homeManagerConfiguration {
      pkgs = packagesFor "x86_64-linux";
      extraSpecialArgs = {
        inherit hunk;
        repoRoot = "/home/michael/dev/nix-config";
      };
      modules = [
        ./home.nix
        ./hosts/ai-dev.nix
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
    homeConfigurations."michael@ai-dev" = aiDev;

    checks.aarch64-darwin = {
      macbook-system = darwinConfig.config.system.build.toplevel;
      macbook-home = darwinConfig.config.home-manager.users.mm.home.activationPackage;
      hunk = hunk.packages.aarch64-darwin.hunk;
      opencode = (packagesFor "aarch64-darwin").opencode;
    };
    checks.x86_64-linux = {
      ai-dev-home = aiDev.activationPackage;
      ai-dev-profile-command = (packagesFor "x86_64-linux").runCommand "ai-dev-profile-command" {} ''
        if grep -q "profile install" ${aiDev.activationPackage}/activate; then
          echo "ai-dev activation uses deprecated nix profile install" >&2
          exit 1
        fi
        touch "$out"
      '';
      ai-dev-maintenance-interface = (packagesFor "x86_64-linux").runCommand "ai-dev-maintenance-interface" {} ''
        maintenance=${aiDev.config.home.path}/bin/ai-dev-maintenance
        test -x "$maintenance"
        if HOME="$TMPDIR/empty-home" "$maintenance" status >status.log 2>&1; then
          echo "status unexpectedly passed without installed tools" >&2
          exit 1
        fi
        for tool in Claude Codex Pi Herdr Moshi OpenCode; do
          grep -q "$tool" status.log
        done

        fake_home="$TMPDIR/fake-home"
        fake_bin="$fake_home/.local/bin"
        mkdir -p "$fake_bin"
        cat >"$fake_bin/fake-tool" <<'SCRIPT'
        #!/usr/bin/env bash
        case "$(basename "$0")" in
          herdr)
            [[ ''${1:-} == integration ]] && { echo "integrations ready"; exit 0; }
            ;;
          moshi-hook)
            [[ ''${1:-} == status ]] && { echo '{}'; exit 0; }
            ;;
          systemctl)
            exit 0
            ;;
          ss)
            echo "LISTEN 0 128 127.0.0.1:24543 0.0.0.0:*"
            exit 0
            ;;
        esac
        echo "$(basename "$0") 1.0.0"
        SCRIPT
        chmod +x "$fake_bin/fake-tool"
        for command in node claude codex pi herdr moshi-hook opencode systemctl ss; do
          ln -s fake-tool "$fake_bin/$command"
        done
        HOME="$fake_home" "$maintenance" status >healthy-status.log
        touch "$out"
      '';
      hunk = hunk.packages.x86_64-linux.hunk;
      opencode = (packagesFor "x86_64-linux").opencode;
    };
  };
}
