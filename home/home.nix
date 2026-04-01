{...}: {
  imports = [
    ./modules
  ];

  home = {
    username = "mm";
    homeDirectory = "/Users/mm";
    stateVersion = "22.05";
    shellAliases = {
      ls = "eza -la";
      cat = "bat";
    };
    # remove message when entering terminal if macos
    file.".hushlogin" = {
      enable = true;
      text = "";
    };
  };

  # allow unfree packages via cli
  xdg.configFile."nixpkgs/config.nix" = {
    enable = true;
    text = ''
      {
        allowUnfree = true;
      }
    '';
  };

  # configure common home-manager modules
  common = {
    neovim.enable = true;
    wezterm.enable = true;
    git = {
      enable = true;
      workProfiles = [
        {
          name = "michaelmbc";
          email = "michaelmbc@users.noreply.github.com";
          directory = "~/work/";
          sshKey = "~/.ssh/github_bc";
        }
      ];
    };
    ssh.enable = true;
    yazi.enable = true;
    karabiner.enable = true;

    # cli
    apps.enable = true;
    fish.enable = true;
    fzf.enable = true;
    zsh.enable = true;
  };

  programs.swaylock.enable = false;
  manual.manpages.enable = false;
  programs.home-manager.enable = true;
}
