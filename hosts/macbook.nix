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

  programs.git = {
    includes = [
      {
        condition = "gitdir:~/businesscraft/";
        contents = {
          user = {
            name = "michaelmbc";
            email = "michaelmbc@users.noreply.github.com";
          };
          core.sshCommand = "ssh -i ~/.ssh/github_bc";
        };
      }
    ];
    settings.user = {
      name = "mich-murphy";
      email = "github@elmurphy.com";
    };
  };
}
