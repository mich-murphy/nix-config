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
    pkgs.tmux
    pkgs.nmap
    pkgs.kubectl
    pkgs.k9s
    pkgs.kubernetes-helm
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
      "hashicorp/tap"
    ];
    # homebrew formulae to install
    brews = [
      "gnu-sed" # neovim requirement
      "siderolabs/tap/talosctl"
    ];
    # homebrew casks for install
    casks = [
      "firefox"
      "wezterm"
      "spaceid" # identify current spaces in menu bar
      "raycast" # launcher
      "1password"
      "stats" # show system stats in menu bar
      "obsidian"
      "zotero@beta" # pdf reading and higlights
      "tailscale"
      "displaylink" # enable connection of lenovo dock
      "karabiner-elements"
      "yubico-yubikey-manager" # manage yubikey
      "utm" # manage virtual machines
      "plexamp"
      "jordanbaird-ice" # menu bar manager
      "rancher"
      "nextcloud"
    ];
  };
}
