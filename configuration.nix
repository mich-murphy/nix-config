{pkgs, ...}: let
  repoRoot = "/Users/mm/dev/nix-config";
in {
  imports = [
    ./darwin
  ];

  _module.args.repoRoot = repoRoot;

  nixpkgs = {
    hostPlatform = "aarch64-darwin";
    config.allowUnfree = true;
  };

  # Nix is managed by the Determinate installer.
  nix.enable = false;

  environment.variables.LESS = "--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4";

  system = {
    primaryUser = "mm";
    stateVersion = 4;
  };

  users.users.mm = {
    home = "/Users/mm";
    shell = pkgs.fish;
    createHome = true;
  };

  programs.fish.enable = true;

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    backupFileExtension = "backup";
    extraSpecialArgs = {inherit repoRoot;};
    users.mm = ./home.nix;
  };
}
