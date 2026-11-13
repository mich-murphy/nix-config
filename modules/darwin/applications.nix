{ pkgs, ... }: {
  environment.variables.HOMEBREW_NO_ENV_HINTS = "1";

  environment.systemPackages = [
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs.gnused
    pkgs.moshi-hook.app
    # The same mas nix-darwin gives `brew bundle` for masApps.
    pkgs.mas
  ];

  # Also installs `op` at /usr/local/bin/op, the only path where the
  # 1Password app's CLI integration finds it.
  programs._1password.enable = true;

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
      extraEnv = {
        HOMEBREW_NO_ENV_HINTS = "1";
        HOMEBREW_SERVICES_NO_DOMAIN_WARNING = "1";
      };
    };
    casks = [
      "claude-code@latest"
      "codex"
      "ghostty"
      "karabiner-elements"
      "whichspace"
      "1password"
      "stats"
      "obsidian"
      "zotero"
      "tailscale-app"
      # Thaw 3 is the only line that supports macOS 27, and it ships only
      # as a beta; the stable cask is Thaw 2, built for macOS 15 and 26.
      "thaw@beta"
      "firefox"
      "google-chrome"
      "iina"
      "plexamp"
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-auto-update"
      "microsoft-teams"
      "orbstack"
      "raycast"
      "linearmouse"
      "wispr-flow"
      "nextcloud"
      "grok-bot"
    ];
    brews = [
      "pi-coding-agent"
      "mole"
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466;
      "Supernote Partner" = 1494992020;
    };
  };
}
