{lib, ...}: {
  home = {
    username = "michael";
    homeDirectory = "/home/michael";

    # /tmp is a quota-limited RAM tmpfs; agent scratch belongs on disk.
    sessionVariables.TMPDIR = "/var/tmp/michael";
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
  ];
}
