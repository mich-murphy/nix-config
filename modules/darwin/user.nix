{ config, pkgs, ... }:
let
  user = config.system.primaryUser;
  firstMacAccountUid = 501;
in
{
  users.knownUsers = [ user ];
  users.users.${user} = {
    uid = firstMacAccountUid;
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
