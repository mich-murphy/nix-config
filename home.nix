{
  config,
  lib,
  ...
}: {
  imports = [
    ./home
  ];

  home = {
    stateVersion = "22.05";
    sessionPath = ["${config.home.homeDirectory}/.local/bin"];

    shellAliases = {
      ls = "eza -la";
      cat = "bat";
    };

    sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
      LESS = "--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4";
    };

    file.".hushlogin".text = "";

    # Home Manager master and nixpkgs-unstable can advertise different
    # development release labels despite being their documented pairing.
    enableNixpkgsReleaseCheck = lib.mkDefault false;
  };

  manual.manpages.enable = false;
  programs.home-manager.enable = true;
}
