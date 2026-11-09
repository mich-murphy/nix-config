# Home Manager modules shared by every Mac.
{
  imports = [
    ./ssh.nix
    ./terminals.nix
    ./karabiner.nix
    ./git.nix
    ./packages.nix
  ];
}
