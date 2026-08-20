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
  environment.variables.HOMEBREW_NO_ENV_HINTS = "1";

  environment.systemPackages = [
    pkgs.xcode-install
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs._1password-cli
    pkgs.gnused
    plannotator
  ];

  fonts.packages = [
    pkgs.nerd-fonts.jetbrains-mono
    pkgs.nerd-fonts._0xproto
  ];

  homebrew = {
    enable = true;
    enableFishIntegration = true;
    onActivation = {
      autoUpdate = true;
      upgrade = true;
      cleanup = "zap";
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
      "firefox"
      "google-chrome"
      "iina"
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-teams"
      "docker-desktop"
      "displaylink"
      "raycast"
      "winbox"
      "xcodes-app"
      "linearmouse"
      "wispr-flow"
    ];
    brews = [
      "herdr"
      "pi-coding-agent"
      "mas"
      "mole"
      "xcodes"
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466;
      "Supernote Partner" = 1494992020;
    };
  };
}
