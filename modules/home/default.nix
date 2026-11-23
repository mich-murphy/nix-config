{ inputs, ... }:
{
  imports = [
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
    shellAliases = {
      ls = "eza -la";
      cat = "bat";
    };

    sessionVariables = {
      LESS = "--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4";
    };

    file.".hushlogin".text = "";
  };

  nix.registry.nixpkgs.flake = inputs.nixpkgs;

  manual.manpages.enable = false;
}
