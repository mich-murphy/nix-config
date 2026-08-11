{hunk, ...}: {
  imports = [
    hunk.homeManagerModules.hunk
    ./shell.nix
    ./cli.nix
    ./git.nix
    ./neovim.nix
    ./yazi.nix
    ./herdr.nix
    ./coding-agents.nix
  ];
}
