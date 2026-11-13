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
  };

  # Determinate's global registry points `nixpkgs` at FlakeHub's
  # nixpkgs-weekly. Pin it to this flake's nixpkgs so `nix shell nixpkgs#…`
  # reuses store paths the system already has.
  nix.registry.nixpkgs.flake = inputs.nixpkgs;

  manual.manpages.enable = false;
}
