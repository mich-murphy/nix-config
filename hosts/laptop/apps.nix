{pkgs, ...}: let
  plannotator = pkgs.stdenvNoCC.mkDerivation {
    pname = "plannotator";
    version = "0.24.1";
    src = pkgs.fetchurl {
      url = "https://github.com/backnotprop/plannotator/releases/download/v0.24.1/plannotator-darwin-arm64";
      hash = "sha256-FzObDbw4fXLIMzeifzmyOQfNK90jYd5TXHpaYzjwzZE=";
    };
    dontUnpack = true;
    dontStrip = true;
    installPhase = ''
      runHook preInstall
      install -Dm755 "$src" "$out/bin/plannotator"
      runHook postInstall
    '';
  };
in {
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
    pkgs.uv
    pkgs.xcode-install
    pkgs.tmux
    pkgs.herdr
    pkgs.mosh
    pkgs.nmap
    pkgs.pi-coding-agent
    pkgs._1password-cli
    pkgs.gnused
    pkgs.just
    pkgs.acli
    pkgs.azure-cli
    pkgs.doctl
    pkgs.dust
    plannotator
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
      "ghostty"
      "wezterm"
      "whichspace" # identify and switch spaces from the menu bar
      "1password"
      "stats" # show system stats in menu bar
      "obsidian"
      "zotero@beta" # pdf reading and higlights
      "tailscale-app"
      "karabiner-elements"
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
      "displaylink"
      "raycast" # launcher
      "winbox"
      "xcodes-app" # install/manage full Xcode with hardware-key auth support
      "linearmouse"
    ];
    brews = [
      "hunk" # terminal diff viewer
      "mas" # required for masApps below to actually install
      "mole" # macOS cleanup and maintenance CLI
      "xcodes" # CLI for installing/selecting Xcode versions
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466; # formerly Microsoft Remote Desktop
      "Supernote Partner" = 1494992020;
      "Azure VPN Client" = 1553936137;
    };
  };
}
