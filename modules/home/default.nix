{
  config,
  inputs,
  ...
}:
{
  imports = [
    inputs.hunk.homeManagerModules.hunk
    ./live-files.nix
    ./shell.nix
    ./cli.nix
    ./git.nix
    ./neovim.nix
    ./yazi.nix
    ./herdr.nix
    ./moshi.nix
    ./agents.nix
  ];

  home = {
    sessionPath = [ "${config.home.homeDirectory}/.local/bin" ];

    shellAliases = {
      ls = "eza -la";
      cat = "bat";
    };

    sessionVariables = {
      LESS = "--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4";
    };

    file.".hushlogin".text = "";

    # Home Manager master and nixpkgs-unstable can advertise different
    # development release labels despite being their documented pairing.
    enableNixpkgsReleaseCheck = false;
  };

  manual.manpages.enable = false;
}
