{ config, lib, ... }:
let
  user = config.system.primaryUser;
  home = config.users.users.${user}.home;
  asUser = ''launchctl asuser "$(id -u -- ${user})" sudo --user=${user} --'';

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

  enabledHotKey = hotKey true;
  disabledHotKey = hotKey false;

  spaceCharacter = 32;
  noCharacter = 65535;
  keyCode = {
    space = 49;
    ansi1 = 18;
    ansi2 = 19;
    ansi3 = 20;
    ansi4 = 21;
    ansi5 = 23;
  };

  symbolicHotKeyId = {
    selectPreviousInputSource = "60";
    selectNextInputSource = "61";
    showSpotlightSearch = "64";
    switchToDesktop1 = "118";
    switchToDesktop2 = "119";
    switchToDesktop3 = "120";
    switchToDesktop4 = "121";
    switchToDesktop5 = "122";
  };

  symbolicHotKeys = with symbolicHotKeyId; {
    ${selectPreviousInputSource} = disabledHotKey [
      spaceCharacter
      keyCode.space
      control
    ];
    ${selectNextInputSource} = disabledHotKey [
      spaceCharacter
      keyCode.space
      (control + option)
    ];
    ${showSpotlightSearch} = disabledHotKey [
      spaceCharacter
      keyCode.space
      command
    ];
    ${switchToDesktop1} = enabledHotKey [
      noCharacter
      keyCode.ansi1
      control
    ];
    ${switchToDesktop2} = enabledHotKey [
      noCharacter
      keyCode.ansi2
      control
    ];
    ${switchToDesktop3} = enabledHotKey [
      noCharacter
      keyCode.ansi3
      control
    ];
    ${switchToDesktop4} = enabledHotKey [
      noCharacter
      keyCode.ansi4
      control
    ];
    ${switchToDesktop5} = enabledHotKey [
      noCharacter
      keyCode.ansi5
      control
    ];
  };

  moshiPhone = {
    tailnetIPv4 = "100.116.176.77";
    sshKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIM9WH/bCZ+uyjIpd/Z2Dx+fs+/RunKTy9aNEHuQRfYGD moshi-phone";
  };

  hotCornerDisabled = 1;
  clockDateAlways = 1;

  symbolicHotKeyArgs = lib.concatStringsSep " " (
    lib.mapAttrsToList (
      id: entry: "${id} ${lib.escapeShellArg (lib.generators.toPlist { escape = true; } entry)}"
    ) symbolicHotKeys
  );
in
{
  networking = {
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

  security.sudo.extraConfig = ''
    Defaults timestamp_timeout=2
  '';

  services.openssh = {
    enable = true;
    extraConfig = ''
      PasswordAuthentication no
      KbdInteractiveAuthentication no
      AuthorizedKeysFile none
      AllowUsers ${user}@${moshiPhone.tailnetIPv4}
      PermitRootLogin no
      PermitUserRC no
      MaxAuthTries 3
      LoginGraceTime 30
      ClientAliveInterval 300
      AllowAgentForwarding no
      AllowStreamLocalForwarding no
      AllowTcpForwarding local
      PermitOpen 127.0.0.1:* localhost:* [::1]:*
    '';
  };

  users.users.${user}.openssh.authorizedKeys.keys = [ moshiPhone.sshKey ];

  system = {
    activationScripts = {
      postActivation.text = ''
        ${asUser} defaults write com.apple.symbolichotkeys AppleSymbolicHotKeys -dict-add ${symbolicHotKeyArgs}

        ${asUser} defaults -currentHost write com.apple.controlcenter AirplayReceiverEnabled -bool false

        ${asUser} defaults -currentHost write com.apple.coreservices.useractivityd ActivityAdvertisingAllowed -bool false
        ${asUser} defaults -currentHost write com.apple.coreservices.useractivityd ActivityReceivingAllowed -bool false

        killall Finder || true
        killall SystemUIServer || true
      '';

      extraActivation.text = ''
        pmset -c sleep 0 displaysleep 10 disksleep 10
        pmset -b sleep 1 displaysleep 2 disksleep 10 powernap 0

        ${asUser} /usr/bin/osascript -l JavaScript -e '
            ObjC.import("Foundation");
            $.NSBundle.bundleWithPath("/System/Library/PrivateFrameworks/PowerUI.framework").load;
            const c = $.NSClassFromString("PowerUISmartChargeClient").alloc.initWithClientName("nix-darwin");
            const ok = (r) => { if (r != 1) throw new Error("PowerUI call failed"); };
            if (c.isMCLCurrentlyEnabled(null) != 1) ok(c.enableMCL(null));
            if (c.getMCLLimitWithError(null) != 80) ok(c.setMCLLimitError(80, null));
            if (c.isSmartChargingCurrentlyEnabled(null) == 1) ok(c.disableSmartCharging(null));
          ' >/dev/null \
          || echo "warning: could not set the battery Charge Limit" >&2
      '';
    };

    defaults = {
      CustomUserPreferences = {
        "com.apple.AdLib".allowApplePersonalizedAdvertising = false;
        "com.apple.assistant.support"."Assistant Enabled" = false;
        "com.apple.desktopservices" = {
          DSDontWriteNetworkStores = true;
          DSDontWriteUSBStores = true;
        };
        "com.apple.finder" = {
          ShowRecentTags = false;
          WarnOnEmptyTrash = false;
        };
        "com.apple.universalcontrol".Disable = true;
        "com.apple.sharingd".DiscoverableMode = "Off";
        "com.apple.screencapture" = {
          location-screenrecording = "~/Pictures";
          location-screenshot = "~/Pictures";
        };
        NSGlobalDomain = {
          AppleLanguages = [ "en-AU" ];
          AppleLocale = "en_AU";
        };
      };
      CustomSystemPreferences."/Library/Preferences/com.apple.SoftwareUpdate" = {
        AutomaticCheckEnabled = true;
        ScheduleFrequency = 1;
        AutomaticDownload = 1;
        CriticalUpdateInstall = 1;
        ConfigDataInstall = 1;
        AutomaticallyInstallAppUpdates = true;
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
        wvous-br-corner = hotCornerDisabled;
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
        CreateDesktop = true;
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
        TrackpadTwoFingerFromRightEdgeSwipeGesture = 0;
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
        ShowDate = clockDateAlways;
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
