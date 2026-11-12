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
      # Also uninstalls undeclared App Store apps, whatever the masApps option
      # text says (https://github.com/nix-darwin/nix-darwin/issues/1860).
      cleanup = "uninstall";
      # brew bundle runs under sudo, so it does not see environment.variables.
      extraEnv = {
        HOMEBREW_NO_ENV_HINTS = "1";
        HOMEBREW_SERVICES_NO_DOMAIN_WARNING = "1";
      };
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
      "zotero"
      "tailscale-app"
      "utm"
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
      # The three Office casks deselect AutoUpdate in their installers.
      "microsoft-auto-update"
      "microsoft-teams"
      "orbstack"
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
