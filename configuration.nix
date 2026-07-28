{
  hunk,
  pkgs,
  ...
}: let
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
    extraSpecialArgs = {inherit hunk repoRoot;};
    users.mm.imports = [
      ./home.nix
      ./hosts/macbook.nix
    ];
  };
}
