{...}: {
  home = {
    username = "michael";
    homeDirectory = "/home/michael";
  };

  programs.git.includes = [
    {
      path = "~/.gitconfig-personal";
    }
    {
      condition = "gitdir:~/work/businesscraft/";
      path = "~/.gitconfig-businesscraft";
    }
  ];
}
