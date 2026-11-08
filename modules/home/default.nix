# Home Manager modules shared by every host.
{
  config,
  inputs,
  lib,
  ...
}:
{
  imports = [
    inputs.hunk.homeManagerModules.hunk
    ./shell.nix
    ./cli.nix
    ./git.nix
    ./neovim.nix
    ./yazi.nix
    ./herdr.nix
    ./agents.nix
  ];

  options.nixConfig.checkout = lib.mkOption {
    type = lib.types.str;
    default = "${config.home.homeDirectory}/dev/nix-config";
    description = ''
      Absolute path of this repository's checkout. Writable configuration is
      linked here with mkOutOfStoreSymlink, so applications can save to it.
    '';
  };

  config = {
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
      enableNixpkgsReleaseCheck = lib.mkDefault false;
    };

    manual.manpages.enable = false;
  };
}
