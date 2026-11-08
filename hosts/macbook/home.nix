{ ... }:
{
  imports = [
    ../../modules/home/mac/ssh.nix
    ../../modules/home/mac/terminals.nix
    ../../modules/home/mac/karabiner.nix
  ];

  # Set once at first activation; never bump it to track releases.
  home.stateVersion = "22.05";

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
