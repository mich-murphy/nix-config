{...}: {
  imports = [
    ./home
  ];

  home = {
    stateVersion = "22.05";
    sessionPath = ["/Users/mm/.local/bin"];

    shellAliases = {
      ls = "eza -la";
      cat = "bat";
    };

    file.".hushlogin".text = "";

    # Home Manager master and nixpkgs-unstable can advertise different
    # development release labels despite being their documented pairing.
    enableNixpkgsReleaseCheck = false;
  };

  manual.manpages.enable = false;
  programs.home-manager.enable = true;
}
