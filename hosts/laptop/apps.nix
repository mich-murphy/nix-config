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
    pkgs.tmux
    pkgs.nmap
    pkgs._1password-cli
    pkgs.gnused
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
      "hashicorp/tap"
      "atlassian/homebrew-acli"
    ];
    # homebrew casks for install
    casks = [
      "wezterm"
      "spaceid" # identify current spaces in menu bar
      "raycast" # launcher
      "1password"
      "stats" # show system stats in menu bar
      "obsidian"
      "zotero@beta" # pdf reading and higlights
      "tailscale-app"
      "displaylink" # enable connection of lenovo dock
      "karabiner-elements"
      "yubico-yubikey-manager" # manage yubikey
      "utm" # manage virtual machines
      "jordanbaird-ice" # menu bar manager
      "owncloud"
      "firefox"
      "google-chrome"
      "iina"
      "claude-code"
      "codex"
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-teams"
      "datagrip"
    ];
    brews = [
      "atlassian/acli/acli"
      "pi-coding-agent"
    ];
    masApps = {
      "Microsoft Remote Desktop" = 1295203466;
      "Supernote Partner" = 1494992020;
    };
  };
}
