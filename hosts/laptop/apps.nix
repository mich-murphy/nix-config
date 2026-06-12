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
    pkgs.curl
    pkgs.python3
    pkgs.xcode-install
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs._1password-cli
    pkgs.gnused
    pkgs.just
    pkgs.acli
  ];

  fonts.packages = [
    pkgs.nerd-fonts.jetbrains-mono
    pkgs.nerd-fonts._0xproto
  ];

  homebrew = {
    enable = true;
    onActivation = {
      # update/upgrade manually with `brew upgrade` — keeps rebuilds fast and deterministic
      autoUpdate = false;
      upgrade = false;
      cleanup = "zap"; # uninstall all formulae and casks not listed in brewfile
      extraFlags = ["--force-cleanup"];
    };
    # homebrew casks for install
    casks = [
      "wezterm"
      "spaceid" # identify current spaces in menu bar
      "1password"
      "stats" # show system stats in menu bar
      "obsidian"
      "zotero@beta" # pdf reading and higlights
      "tailscale-app"
      "karabiner-elements"
      "yubico-yubikey-manager" # manage yubikey
      "utm" # manage virtual machines
      "jordanbaird-ice" # menu bar manager
      "owncloud"
      "firefox"
      "google-chrome"
      "iina"
      "claude-code@latest"
      "codex"
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-teams"
      "datagrip"
      "docker-desktop"
      "sol" # launcher
      "winbox"
    ];
    # raw Brewfile line: nix-darwin's cask args schema lacks `adopt`, needed
    # because the app pre-dates cask management and brew refuses to overwrite it
    extraConfig = ''
      cask "qmk-toolbox", args: { adopt: true }
    '';
    brews = [
      "pi-coding-agent"
      "mas" # required for masApps below to actually install
    ];
    masApps = {
      "Windows App" = 1295203466; # formerly Microsoft Remote Desktop
      "Supernote Partner" = 1494992020;
      "Azure VPN Client" = 1553936137;
    };
  };
}
