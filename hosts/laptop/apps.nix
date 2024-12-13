{pkgs, ...}: {
  # installation of apps and packages
  # nix-darwin options documentation: https://daiderd.com/nix-darwin/manual/index.html#sec-options

  imports = [
    ../../darwin/modules
  ];

  common = {
    yabai.enable = true;
    skhd.enable = true;
  };

  environment.systemPackages = [
    pkgs.git
    pkgs.curl
    pkgs.xcode-install
    pkgs.deploy-rs
  ];

  fonts.packages = [
    pkgs.nerd-fonts.jetbrains-mono
  ];

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
    ];
    # homebrew formulae to install
    brews = [
      "gnu-sed" # neovim requirement
    ];
    # homebrew casks for install
    casks = [
      "firefox@nightly"
      "firefox"
      "wezterm"
      "spaceid" # identify current spaces in menu bar
      "raycast" # launcher
      "1password"
      "stats" # show system stats in menu bar
      "obsidian"
      "zotero@beta" # pdf reading and higlights
      "tailscale"
      "nextcloud"
      "displaylink" # enable connection of lenovo dock
      "karabiner-elements"
      "yubico-yubikey-manager" # manage yubikey
      "utm" # manage virtual machines
      "plexamp"
      "moonlight"
      "jordanbaird-ice" # menu bar manager
    ];
  };
}
