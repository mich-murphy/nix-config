{hunk, ...}: {
  imports = [
    hunk.homeManagerModules.hunk
    ./shell.nix
    ./cli.nix
    ./git.nix
    ./yazi.nix
    ./herdr.nix
    ./coding-agents.nix
  ];
}
