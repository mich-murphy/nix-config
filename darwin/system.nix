{...}: {
  networking = {
    computerName = "macbook";
    hostName = "macbook";
    dns = ["100.100.100.100" "1.1.1.1" "1.0.0.1"];
    knownNetworkServices = ["Wi-Fi" "Thunderbolt Bridge"];
  };

  security.pam.services.sudo_local = {
    touchIdAuth = true;
    reattach = true;
  };

  networking.applicationFirewall = {
    enable = true;
    allowSigned = true;
    allowSignedApp = true;
    enableStealthMode = true;
  };

  system.activationScripts.postActivation.text = ''
    killall Dock || true
    killall Finder || true
    killall SystemUIServer || true
  '';

  # Pin power management via pmset because power.sleep cannot set separate
  # values for AC and battery power.
  system.activationScripts.extraActivation.text = ''
    pmset -c sleep 1 displaysleep 10 disksleep 10
    pmset -b sleep 1 displaysleep 2 disksleep 10
  '';

  system = {
    checks.verifyNixPath = false;
    defaults = {
      CustomUserPreferences = {
        "com.apple.BluetoothAudioAgent"."Apple Bitpool Min (editable)" = 40;
        "com.apple.AdLib".allowApplePersonalizedAdvertising = false;
        "com.apple.desktopservices" = {
          DSDontWriteNetworkStores = true;
          DSDontWriteUSBStores = true;
        };
        "com.apple.SoftwareUpdate" = {
          AutomaticCheckEnabled = true;
          ScheduleFrequency = 1;
          AutomaticDownload = 1;
          CriticalUpdateInstall = 1;
        };
      };
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
        wvous-br-corner = 1;
      };
      finder = {
        AppleShowAllExtensions = true;
        AppleShowAllFiles = true;
        CreateDesktop = false;
        FXPreferredViewStyle = "Nlsv";
        FXEnableExtensionChangeWarning = false;
        QuitMenuItem = true;
        ShowPathbar = true;
        _FXShowPosixPathInTitle = true;
        _FXSortFoldersFirst = true;
        FXDefaultSearchScope = "SCcf";
      };
      loginwindow = {
        DisableConsoleAccess = true;
        GuestEnabled = false;
      };
      screencapture = {
        location = "~/Pictures";
        type = "png";
        disable-shadow = true;
      };
      trackpad.FirstClickThreshold = 0;
      NSGlobalDomain = {
        AppleEnableMouseSwipeNavigateWithScrolls = false;
        AppleEnableSwipeNavigateWithScrolls = false;
        AppleKeyboardUIMode = 3;
        ApplePressAndHoldEnabled = false;
        AppleInterfaceStyle = "Dark";
        InitialKeyRepeat = 15;
        KeyRepeat = 2;
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
      menuExtraClock = {
        Show24Hour = true;
        ShowDate = 1;
        ShowDayOfWeek = true;
        ShowSeconds = false;
      };
      spaces.spans-displays = false;
      WindowManager.EnableStandardClickToShowDesktop = false;
      LaunchServices.LSQuarantine = false;
    };
    keyboard.enableKeyMapping = true;
  };
}
