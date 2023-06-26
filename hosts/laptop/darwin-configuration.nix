{ config, pkgs, user, host, ... }:

{
  imports = [
    ../../common/darwin
  ];

  nixpkgs.config.allowUnfree = true;

  networking = {
    computerName = "${host}";
    hostName = "${host}";
    dns = [ "100.100.100.100" "1.1.1.1" "1.0.0.1" ];
    knownNetworkServices = [ "Wi-Fi" "Thunderbolt Bridge" ];
  };

  users.users."${user}" = {
    shell = pkgs.zsh;
    home = "/Users/${user}";
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
      (nerdfonts.override { fonts = [ "JetBrainsMono" ]; })
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
      "homebrew/core"
      "homebrew/cask"
      "homebrew/cask-versions"
      "homebrew/cask-drivers"
      "homebrew/services"
    ];
    brews = [
      "gcc"
    ];
    casks = [
      "firefox"
      "alacritty"
      "spaceid"
      "1password"
      "alfred"
      "stats"
      "roon"
      "karabiner-elements"
      "obsidian"
      "tailscale"
    ];
  };

  common.yabai.enable = true;

  programs.zsh.enable = true;

  services = {
    nix-daemon.enable = true;
    lorri.enable = true;
  };

  nix = {
    package = pkgs.nixUnstable;
    gc = {
      automatic = true;
      interval.Day = 7;
      options = "--delete-older-than 7d";
    };
    settings = {
      trusted-users = [ "@admin" "${user}" ];
      auto-optimise-store = true;
      substituters = [
        "https://nix-community.cachix.org"
        "https://mich-murphy.cachix.org"
      ];
      trusted-public-keys = [
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "mich-murphy.cachix.org-1:N3hbImjsw9hwKSDZasLevmQCdQBoLabHgeq+LuSvLhs="
      ];
    };
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };

  security.pam.enableSudoTouchIdAuth = true;

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
    };
    keyboard = {
      enableKeyMapping = true;  # needed for skhd
      # remapCapsLockToControl = true;
    };
    stateVersion = 4;
  };
}
