{ pkgs, ... }: {
  environment.variables.HOMEBREW_NO_ENV_HINTS = "1";

  environment.systemPackages = [
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs._1password-cli
    pkgs.gnused
    pkgs.moshi-hook.app
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
      cleanup = "uninstall";
    };
    casks = [
      # nixpkgs azure-cli depends on fabric, which is marked broken with
      # invoke >= 3 (https://github.com/fabric/fabric/issues/2376).
      "azure-cli"
      "claude-code@latest"
      "codex"
      "ghostty"
      "karabiner-elements"
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
      "pi-coding-agent"
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
