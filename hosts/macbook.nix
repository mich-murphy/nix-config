{...}: {
  imports = [
    ../home/ssh.nix
    ../home/neovim.nix
    ../home/terminals.nix
    ../home/karabiner.nix
  ];

  home = {
    username = "mm";
    homeDirectory = "/Users/mm";
  };
}
