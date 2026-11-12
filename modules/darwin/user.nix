{ config, pkgs, ... }:
let
  user = config.system.primaryUser;
in
{
  users.users.${user} = {
    home = "/Users/${user}";
    shell = pkgs.fish;
  };
  programs.fish.enable = true;

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
  };
}
