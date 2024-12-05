{pkgs, ...}: {
  imports = [
    ./modules
  ];

  home = {
    username = "mm";
    homeDirectory = "/Users/mm";
    stateVersion = "22.05";
    packages = with pkgs; [
      _1password-cli
    ];
    # remove message when entering terminal if macos
    file.".hushlogin" = {
      enable = true;
      text = "";
    };
  };

  # allow unfree packages via cli
  xdg.configFile."nixpkgs/config.nix" = {
    enable = true;
    target = "nixpkgs/config.nix";
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
    git.enable = true;
    ssh.enable = true;
    yazi.enable = true;
    karabiner.enable = true;
    # cli
    apps.enable = true;
    fzf.enable = true;
    zsh.enable = true;
  };

  programs.home-manager.enable = true;
}
