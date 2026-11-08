# nix-darwin modules shared by every Mac.
{
  imports = [
    ./system.nix
    ./maintenance.nix
    ./applications.nix
    ./window-management.nix
  ];
}
