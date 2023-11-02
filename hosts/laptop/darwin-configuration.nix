{
  pkgs,
  inputs,
  ...
}: {
  imports = [
    ../../common/darwin
  ];

  networking = {
    computerName = "macbook";
    hostName = "macbook";
    dns = ["100.100.100.100" "1.1.1.1" "1.0.0.1"];
    knownNetworkServices = ["Wi-Fi" "Thunderbolt Bridge"];
  };

  users.users."mm" = {
    shell = pkgs.zsh;
    home = "/Users/mm";
    createHome = true;
  };

  environment = {
    systemPackages = with pkgs; [
      git
      curl
      xcode-install
    ];
  };

  fonts = {
    fontDir.enable = true;
    fonts = with pkgs; [
      # https://github.com/NixOS/nixpkgs/blob/nixos-23.05/pkgs/data/fonts/nerdfonts/shas.nix
      (nerdfonts.override {fonts = ["JetBrainsMono"];})
    ];
  };

  homebrew = {
    enable = true;
    onActivation = {
      autoUpdate = true;
      upgrade = true;
      cleanup = "zap";
    };
    taps = [
      "homebrew/services"
    ];
    brews = [
      "spotifyd"
      "spotify-tui"
    ];
    casks = [
      "firefox"
      "kitty"
      "spaceid"
      "raycast"
      "1password"
      "stats"
      "obsidian"
      "zotero"
      "tailscale"
      "nextcloud"
      "displaylink"
      "karabiner-elements"
    ];
  };

  common.yabai.enable = true;
  programs.zsh.enable = true;
  services.nix-daemon.enable = true;
  security.pam.enableSudoTouchIdAuth = true;
  nixpkgs.config.allowUnfree = true;
  nixpkgs.hostPlatform = "aarch64-darwin";

  nix = {
    package = pkgs.nixUnstable;
    registry.nixpkgs.flake = inputs.nixpkgs;
    gc = {
      automatic = true;
      interval.Day = 7;
      options = "--delete-older-than 7d";
    };
    settings = {
      trusted-users = ["@admin" "mm"];
      auto-optimise-store = true;
      substituters = [
        "https://nix-community.cachix.org"
      ];
      trusted-public-keys = [
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      ];
    };
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  system = {
    checks.verifyNixPath = false;
    defaults = {
      dock = {
        autohide = true;
        autohide-delay = 0.1;
        autohide-time-modifier = 0.1;
        orientation = "left";
        mru-spaces = false;
        launchanim = false;
        mineffect = "scale";
        tilesize = 48;
        show-recents = false;
      };
      finder = {
        AppleShowAllExtensions = true;
        AppleShowAllFiles = true;
        CreateDesktop = false;
        FXPreferredViewStyle = "Nlsv";
        FXEnableExtensionChangeWarning = false;
        QuitMenuItem = true;
        ShowPathbar = true;
      };
      loginwindow = {
        DisableConsoleAccess = true;
        GuestEnabled = false;
      };
      screencapture = {
        location = "~/Pictures";
      };
      trackpad = {
        FirstClickThreshold = 0;
      };
      NSGlobalDomain = {
        AppleEnableMouseSwipeNavigateWithScrolls = false;
        AppleEnableSwipeNavigateWithScrolls = false;
        AppleKeyboardUIMode = 3;
        ApplePressAndHoldEnabled = false;
        AppleInterfaceStyle = "Dark";
        InitialKeyRepeat = 15;
        KeyRepeat = 1;
        NSAutomaticCapitalizationEnabled = false;
        NSAutomaticDashSubstitutionEnabled = false;
        NSAutomaticPeriodSubstitutionEnabled = false;
        NSAutomaticQuoteSubstitutionEnabled = false;
        NSAutomaticSpellingCorrectionEnabled = false;
        NSAutomaticWindowAnimationsEnabled = false;
        NSDocumentSaveNewDocumentsToCloud = false;
        NSNavPanelExpandedStateForSaveMode = true;
        NSNavPanelExpandedStateForSaveMode2 = true;
      };
      LaunchServices.LSQuarantine = false; # disables "Are you sure?" for new apps
    };
    keyboard = {
      enableKeyMapping = true; # needed for skhd
      remapCapsLockToControl = true;
    };
    stateVersion = 4;
  };
}
