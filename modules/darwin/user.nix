{ config, pkgs, ... }:
let
  user = config.system.primaryUser;
in
{
  # nix-darwin writes a user's login shell only for users it knows, which
  # needs their uid. For an existing account activation also sets
  # PrimaryGroupID (default 20, staff) and nothing else; a uid that does not
  # match is skipped with a warning. 501 is the first account on a Mac.
  users.knownUsers = [ user ];
  users.users.${user} = {
    uid = 501;
    home = "/Users/${user}";
    shell = pkgs.fish;
  };
  programs.fish.enable = true;
  environment.shells = [ pkgs.fish ];

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
  };
}
