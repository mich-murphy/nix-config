{lib, ...}: {
  home = {
    username = "michael";
    homeDirectory = "/home/michael";
  };

  programs.fish.interactiveShellInit = lib.mkBefore ''
    fish_add_path --prepend "$HOME/.nix-profile/bin"
  '';

  programs.starship.settings = {
    hostname.disabled = true;
    username.disabled = true;
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
