{...}: {
  # macos system configuration
  # nix-darwin options documentation: https://daiderd.com/nix-darwin/manual/index.html#sec-options

  security.pam.services.sudo_local.touchIdAuth = true; # use touchid for sudo authentication

  system = {
    primaryUser = "mm";
    checks.verifyNixPath = false; # run NIX_PATH validation checks
    defaults = {
      CustomUserPreferences = {
        "com.apple.AdLib" = {
          allowApplePersonalizedAdvertising = false;
        };
        "com.apple.desktopservices" = {
          # Avoid creating .DS_Store files on network or USB volumes
          DSDontWriteNetworkStores = true;
          DSDontWriteUSBStores = true;
        };
        "com.apple.SoftwareUpdate" = {
          AutomaticCheckEnabled = true;
          ScheduleFrequency = 1; # Check for software updates daily, not just once per week
          AutomaticDownload = 1; # Download newly available updates in background
          CriticalUpdateInstall = 1; # Install System data files & security updates
        };
      };
      dock = {
        autohide = true;
        autohide-delay = 0.1; # configure autohide timing
        autohide-time-modifier = 0.1;
        orientation = "left"; # position of dock
        mru-spaces = false; # re-arrange spaces based on usage
        launchanim = false; # turn off launch animations
        mineffect = "scale"; # minimise animation
        tilesize = 48; # size of dock icons
        show-recents = false; # disable recent apps
      };
      finder = {
        AppleShowAllExtensions = true; # show all file extensions
        AppleShowAllFiles = true; # show hidden files
        CreateDesktop = false; # show icons on desktop
        FXPreferredViewStyle = "Nlsv"; # prefer list view in finder
        FXEnableExtensionChangeWarning = false; # disable changing file extension warning
        QuitMenuItem = true; # enable quit menu item
        ShowPathbar = true; # show filpaths
        _FXShowPosixPathInTitle = true; # show full filepath in titlebar
      };
      loginwindow = {
        DisableConsoleAccess = true; # disable console access from login window
        GuestEnabled = false; # disable guest account
      };
      screencapture = {
        location = "~/Pictures"; # screenshot path
      };
      trackpad = {
        FirstClickThreshold = 0; # force feedback level
      };
      NSGlobalDomain = {
        AppleEnableMouseSwipeNavigateWithScrolls = false; # two finger swipe for forwards/backwards
        AppleEnableSwipeNavigateWithScrolls = false; # same as above
        AppleKeyboardUIMode = 3; # enable full keyboard control
        ApplePressAndHoldEnabled = false; # enable press and hold feature
        AppleInterfaceStyle = "Dark"; # dark mode
        InitialKeyRepeat = 15; # set minimum key repeat time: 15 (225ms) - 120 (1800ms)
        KeyRepeat = 2; # set speed of repeats after first start: 2 (30ms) - 120 (1800ms)
        NSAutomaticCapitalizationEnabled = false;
        NSAutomaticDashSubstitutionEnabled = false;
        NSAutomaticPeriodSubstitutionEnabled = false;
        NSAutomaticQuoteSubstitutionEnabled = false;
        NSAutomaticSpellingCorrectionEnabled = false;
        NSAutomaticWindowAnimationsEnabled = false;
        NSDocumentSaveNewDocumentsToCloud = false; # disable automatic saving of documents to iCloud
        NSNavPanelExpandedStateForSaveMode = true; # expand save panel options by default
        NSNavPanelExpandedStateForSaveMode2 = true;
      };
      LaunchServices.LSQuarantine = false; # disables "Are you sure?" for new apps
    };
    keyboard = {
      enableKeyMapping = true; # needed for skhd
      remapCapsLockToControl = true;
    };
  };
}
