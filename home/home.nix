{...}: {
  imports = [
    ./modules
  ];

  home = {
    username = "mm";
    homeDirectory = "/Users/mm";
    stateVersion = "22.05";
    sessionPath = [
      "/Users/mm/.local/bin"
    ];
    # HM master labels 26.05 while tracking nixpkgs-unstable (26.11); skew is benign.
    enableNixpkgsReleaseCheck = false;
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

  # Ghostty local app config. WezTerm remains the Nix-managed terminal; this
  # mirrors the relevant WezTerm UX for testing the local Ghostty build.
  xdg.configFile."ghostty/config".text = ''
    font-family = Berkeley Mono
    font-size = 13
    theme = Carbonfox
    window-decoration = false
    macos-titlebar-style = transparent
    window-theme = dark

    keybind = ctrl+a>[=copy_mode_enter
  '';

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
          directory = "~/businesscraft/";
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

  manual.manpages.enable = false;
  programs.home-manager.enable = true;
}
