{
  config,
  inputs,
  lib,
  ...
}:
{
  imports = [
    ./live-files.nix
    ./theme.nix
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

  assertions = lib.mapAttrsToList (
    name: entry:
    let
      missing = lib.filter (other: !(config.home.activation ? ${other})) (entry.after ++ entry.before);
    in
    {
      assertion = missing == [ ];
      message = "home.activation.${name} is ordered against ${lib.concatStringsSep ", " missing}, which does not exist and which Home Manager silently ignores";
    }
  ) config.home.activation;

  manual.manpages.enable = false;
}
