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
  environment.systemPackages = [
    pkgs.curl
    pkgs.python3
    pkgs.uv
    pkgs.xcode-install
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
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
      autoUpdate = false;
      upgrade = false;
      cleanup = "zap";
      extraFlags = ["--force-cleanup"];
    };
    casks = [
      "claude-code@latest"
      "codex"
      "ghostty"
      "karabiner-elements"
      "wezterm"
      "whichspace"
      "1password"
      "stats"
      "obsidian"
      "zotero@beta"
      "tailscale-app"
      "utm"
      "jordanbaird-ice"
      "owncloud"
      "firefox"
      "google-chrome"
      "iina"
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-teams"
      "datagrip"
      "docker-desktop"
      "displaylink"
      "raycast"
      "winbox"
      "xcodes-app"
      "linearmouse"
    ];
    brews = [
      "pi-coding-agent"
      "hunk"
      "mas"
      "mole"
      "xcodes"
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466;
      "Supernote Partner" = 1494992020;
      "Azure VPN Client" = 1553936137;
    };
  };
}
