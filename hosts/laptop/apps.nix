{pkgs, ...}: {
  # installation of apps and packages
  # nix-darwin options documentation: https://daiderd.com/nix-darwin/manual/index.html#sec-options

  imports = [
    ../../common/darwin
  ];

  common = {
    yabai.enable = true;
    skhd.enable = true;
  };

  environment.systemPackages = [
    pkgs.git
    pkgs.curl
    pkgs.xcode-install
  ];

  fonts = {
    fontDir.enable = true;
    fonts = [
      # https://github.com/NixOS/nixpkgs/blob/nixos-23.05/pkgs/data/fonts/nerdfonts/shas.nix
      (pkgs.nerdfonts.override {fonts = ["JetBrainsMono"];}) # install nerd font
    ];
  };

  homebrew = {
    enable = true;
    onActivation = {
      autoUpdate = true; # update homebrew and formulae on nix-darwin-rebuild
      upgrade = true; # upgrade during nix-darwin-rebuild
      cleanup = "zap"; # uninstall all formulae and casks not listed in brewfile
    };
    # homebrew repositories to tap
    taps = [
      "homebrew/services"
      "homebrew/cask-versions"
    ];
    # homebrew formulae to install
    brews = [
      "spotifyd" # lightweight spotify daemon
      "spotify-tui"
      "gnu-sed" # neovim requirement
    ];
    # homebrew casks for install
    casks = [
      "firefox"
      "kitty"
      "spaceid" # identify current spaces in menu bar
      "raycast" # launcher
      "1password"
      "stats" # show system stats in menu bar
      "obsidian"
      "zotero-beta" # pdf reading and higlights
      "tailscale"
      "nextcloud"
      "displaylink" # enable connection of lenovo dock
      "karabiner-elements"
      "bartender"
      "yubico-yubikey-manager" # manage yubikey
      "hammerspoon"
      "utm" # manage virtual machines
      "moonlight" # game streaming
      "logitech-options" # management of mouse
    ];
  };
}
