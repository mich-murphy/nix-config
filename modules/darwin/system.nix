{ config, lib, ... }:
let
  user = config.system.primaryUser;
  home = config.users.users.${user}.home;

  control = 262144;
  option = 524288;
  command = 1048576;

  hotKey = enabled: parameters: {
    inherit enabled;
    value = {
      type = "standard";
      inherit parameters;
    };
  };

  # Keyed by com.apple.symbolichotkeys id; parameters are the character, the
  # key code and the modifier mask. Carbon Events.h names key codes 18/19/20/21/23
  # as ANSI 1/2/3/4/5 and 49 as Space.
  symbolicHotKeys = {
    # Input source switching on Ctrl+Space and Ctrl+Option+Space
    "60" = hotKey false [
      32
      49
      control
    ];
    "61" = hotKey false [
      32
      49
      (control + option)
    ];
    # Spotlight on Cmd+Space
    "64" = hotKey false [
      32
      49
      command
    ];
    # Switch to Desktop 1 to 5 on Ctrl+1 to Ctrl+5
    "118" = hotKey true [
      65535
      18
      control
    ];
    "119" = hotKey true [
      65535
      19
      control
    ];
    "120" = hotKey true [
      65535
      20
      control
    ];
    "121" = hotKey true [
      65535
      21
      control
    ];
    "122" = hotKey true [
      65535
      23
      control
    ];
  };

  symbolicHotKeyArgs = lib.concatStringsSep " " (
    lib.mapAttrsToList (
      id: entry: "${id} ${lib.escapeShellArg (lib.generators.toPlist { escape = true; } entry)}"
    ) symbolicHotKeys
  );
in
{
  networking = {
    # Tailscale's app installs its own resolver for the tailnet's names, so
    # 100.100.100.100 is not listed here.
    dns = [
      "1.1.1.1"
      "1.0.0.1"
    ];
    knownNetworkServices = [
      "Wi-Fi"
      "Thunderbolt Bridge"
    ];

    applicationFirewall = {
      enable = true;
      allowSigned = true;
      allowSignedApp = true;
      enableStealthMode = true;
    };
  };

  security.pam.services.sudo_local = {
    touchIdAuth = true;
    reattach = true;
  };

  # launchd binds sshd on every interface, so AllowUsers limits it to the tailnet
  services.openssh = {
    enable = true;
    extraConfig = ''
      PasswordAuthentication no
      KbdInteractiveAuthentication no
      AllowUsers ${config.system.primaryUser}@100.64.0.0/10 ${config.system.primaryUser}@fd7a:115c:a1e0::/48
    '';
  };

  system = {
    activationScripts = {
      # `defaults write` replaces a whole top-level key, so declaring
      # AppleSymbolicHotKeys through system.defaults would drop every shortcut
      # not listed here. -dict-add merges these ids into the rest.
      #
      # AirPlay Receiver is a per-host preference, which system.defaults cannot
      # write.
      postActivation.text = ''
        launchctl asuser "$(id -u -- ${user})" sudo --user=${user} -- \
          defaults write com.apple.symbolichotkeys AppleSymbolicHotKeys -dict-add ${symbolicHotKeyArgs}

        launchctl asuser "$(id -u -- ${user})" sudo --user=${user} -- \
          defaults -currentHost write com.apple.controlcenter AirplayReceiverEnabled -bool false

        killall Finder || true
        killall SystemUIServer || true
      '';

      # Pin power management via pmset because power.sleep cannot set separate
      # values for AC and battery power. On AC the Mac never idle-sleeps, so it
      # stays reachable over the tailnet; a closed lid still sleeps it.
      extraActivation.text = ''
        pmset -c sleep 0 displaysleep 10 disksleep 10
        pmset -b sleep 1 displaysleep 2 disksleep 10 powernap 0
      '';
    };

    defaults = {
      CustomUserPreferences = {
        "com.apple.AdLib".allowApplePersonalizedAdvertising = false;
        # Siri off
        "com.apple.assistant.support"."Assistant Enabled" = false;
        "com.apple.desktopservices" = {
          DSDontWriteNetworkStores = true;
          DSDontWriteUSBStores = true;
        };
        "com.apple.finder" = {
          ShowRecentTags = false;
          WarnOnEmptyTrash = false;
        };
        # screencapture on macOS 27 names only these keys; `location` below is
        # the older name for the same folder.
        "com.apple.screencapture" = {
          location-screenrecording = "~/Pictures";
          location-screenshot = "~/Pictures";
        };
        NSGlobalDomain = {
          AppleLanguages = [ "en-AU" ];
          AppleLocale = "en_AU";
        };
      };
      # softwareupdated reads the system domain, and this option takes the
      # domain verbatim as root, so it needs the full path.
      CustomSystemPreferences."/Library/Preferences/com.apple.SoftwareUpdate" = {
        AutomaticCheckEnabled = true;
        ScheduleFrequency = 1;
        AutomaticDownload = 1;
        CriticalUpdateInstall = 1;
        ConfigDataInstall = 1;
      };
      SoftwareUpdate.AutomaticallyInstallMacOSUpdates = true;
      dock = {
        autohide = true;
        autohide-delay = 0.1;
        autohide-time-modifier = 0.1;
        expose-group-apps = true;
        orientation = "left";
        mru-spaces = false;
        launchanim = false;
        mineffect = "scale";
        tilesize = 48;
        show-recents = false;
        wvous-br-corner = 1; # Disabled (nix-darwin hot-corner enum).
        # A switch rewrites both lists, so change the Dock here.
        persistent-apps = [
          "/Applications/Google Chrome.app"
          "/Applications/Slack.app"
          "/Applications/Obsidian.app"
          "/Applications/Zotero.app"
          "/System/Applications/Messages.app"
          "/System/Applications/Mail.app"
          "/System/Applications/System Settings.app"
          "/Applications/1Password.app"
          "/Applications/Firefox.app"
        ];
        persistent-others = [
          {
            folder = {
              path = "${home}/Downloads/";
              arrangement = "date-added";
              displayas = "stack";
              showas = "fan";
            };
          }
        ];
      };
      finder = {
        AppleShowAllExtensions = true;
        AppleShowAllFiles = true;
        CreateDesktop = false;
        FXPreferredViewStyle = "Nlsv";
        FXEnableExtensionChangeWarning = false;
        FXRemoveOldTrashItems = true;
        NewWindowTarget = "Other";
        NewWindowTargetPath = "file://${home}/Downloads/";
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
      trackpad = {
        ActuateDetents = false;
        FirstClickThreshold = 0;
        ForceSuppressed = true;
        TrackpadTwoFingerFromRightEdgeSwipeGesture = 0; # Disable the Notification Center gesture.
      };
      NSGlobalDomain = {
        AppleEnableMouseSwipeNavigateWithScrolls = false;
        AppleEnableSwipeNavigateWithScrolls = false;
        AppleKeyboardUIMode = 2;
        ApplePressAndHoldEnabled = false;
        AppleInterfaceStyle = "Dark";
        AppleReduceDesktopTinting = true;
        AppleScrollerPagingBehavior = true;
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
        "com.apple.trackpad.forceClick" = false;
        "com.apple.trackpad.scaling" = 1.0;
      };
      menuExtraClock = {
        Show24Hour = true;
        ShowDate = 1; # Always show the full date, not only when space allows.
        ShowDayOfWeek = true;
        ShowSeconds = false;
      };
      spaces.spans-displays = false;
      WindowManager = {
        GloballyEnabled = false;
        EnableStandardClickToShowDesktop = false;
        EnableTilingByEdgeDrag = false;
        EnableTopTilingByEdgeDrag = false;
        EnableTilingOptionAccelerator = false;
        EnableTiledWindowMargins = false;
      };
    };
  };
}
