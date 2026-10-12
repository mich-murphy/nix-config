{pkgs, ...}: {
  environment.variables.HOMEBREW_NO_ENV_HINTS = "1";

  environment.systemPackages = [
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs._1password-cli
    pkgs.gnused
  ];

  fonts.packages = [
    pkgs.nerd-fonts.jetbrains-mono
    pkgs.nerd-fonts.monaspace
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
    taps = [
      {
        name = "rjyo/moshi";
        trusted = true;
      }
    ];
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
      "plexamp"
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-teams"
      "docker-desktop"
      "displaylink"
      "raycast"
      "winbox"
      "linearmouse"
      "wispr-flow"
      "chatgpt"
      "nextcloud"
    ];
    brews = [
      "herdr"
      "pi-coding-agent"
      {
        name = "moshi-hook";
        start_service = true;
        restart_service = "changed";
      }
      "mas"
      "mole"
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466;
      "Supernote Partner" = 1494992020;
    };
  };
}
