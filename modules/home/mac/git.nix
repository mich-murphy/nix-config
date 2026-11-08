# Personal identity on the Macs, with the work identity under ~/businesscraft/.
{
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
