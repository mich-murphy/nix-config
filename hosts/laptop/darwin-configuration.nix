{ config, pkgs, user, host, ... }:

let
  yabai = pkgs.yabai.overrideAttrs (old: rec {
    version = "5.0.2";
    src = builtins.fetchTarball {
      url = https://github.com/koekeishiya/yabai/releases/download/v5.0.2/yabai-v5.0.2.tar.gz;
      sha256 = "0rfg6kqhnsryclny5drj85h442kz5bc9rks60c3lz0a842yvi1c2";
    };
  });
in
{
  nixpkgs.config.allowUnfree = true;

  networking = {
    computerName = "${host}";
    hostName = "${host}";
    dns = [ "9.9.9.9" "149.112.112.112"];
    knownNetworkServices = [ "Wi-Fi" "Thunderbolt Bridge" ];
  };

  users.users."${user}" = {
    shell = pkgs.zsh;
    home = "/Users/${user}";
    createHome = true;
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
      ];
      trusted-public-keys = [
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
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
	InitialKeyRepeat = 10;
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
      enableKeyMapping = true;
      # remapCapsLockToControl = true;
    };
    stateVersion = 4;
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
    casks = [
      "firefox"
      "alacritty"
      "spaceid"
      "1password"
      "alfred"
      "stats"
      "roon"
      "utm"
      "karabiner-elements"
    ];
  };

  programs.zsh.enable = true;

  services = {
    nix-daemon.enable = true;
    tailscale.enable = true;
    lorri.enable = true;
    yabai = {
      enable = true; 
      enableScriptingAddition = true;
      package = yabai;
      config = {
        focus_follows_mouse = "off";
        mouse_follows_focus = "off";
        mouse_modifier = "fn";
        mouse_action1 = "move";
        mouse_action2 = "resize";
        layout = "bsp";
        split_ratio = 0.5;
        auto_balance = "off";
        top_padding = 5;
        bottom_padding = 5;
        left_padding = 5;
        right_padding = 5;
        window_shadow = "float";
        window_gap = 5;
        window_placement = "second_child";
        extraConfig = ''
        yabai -m rule --add title='Preferences' manage=off layer=above
        yabai -m rule --add title='^(Opening)' manage=off layer=above
        yabai -m rule --add title='Library' manage=off layer=above
        yabai -m rule --add app='^Calculator$' manage=off layer=above
        yabai -m rule --add app='^App Store$' manage=off layer=above
        yabai -m rule --add app='^System Preferences$' manage=off layer=above
        yabai -m rule --add app='^Activity Monitor$' manage=off layer=above
        yabai -m rule --add app='Finder' manage=off layer=above
        yabai -m rule --add app='Alfred' manage=off layer=above
        yabai -m rule --add app='1Password' manage=off layer=above
        yabai -m rule --add app='^System Information$' manage=off layer=above
        '';
      };
    };
    skhd = {
      enable = true;
      skhdConfig = ''
        # Applications Shortcuts
        cmd - return : /Applications/Alacritty.App/Contents/MacOS/alacritty
        cmd + shift - return : /Applications/Firefox.App/Contents/MacOS/firefox
        # Toggle Window
        lalt - t : yabai -m window --toggle float; yabai -m window --grid 4:4:1:1:2:2
        lalt - f : yabai -m window --toggle zoom-fullscreen
        lalt + shift - f : yabai -m window --toggle native-fullscreen
        lalt - q : yabai -m window --close
        # Toggle Gaps
        lalt - g : yabai -m space --toggle padding; yabai -m space --toggle gap
        # Focus Window
        lalt - k : yabai -m window --focus north
        lalt - j : yabai -m window --focus south
        lalt - h : yabai -m window --focus west
        lalt - l : yabai -m window --focus east
        # Swap Window
        shift + lalt - k : yabai -m window --swap north
        shift + lalt - j : yabai -m window --swap south
        shift + lalt - h : yabai -m window --swap west
        shift + lalt - l : yabai -m window --swap east
        # Resize Window
        lalt + cmd - h : yabai -m window --resize left:-50:0; yabai -m window --resize right:-50:0
        lalt + cmd - l : yabai -m window --resize left:50:0; yabai -m window --resize right:50:0
        lalt + cmd - k : yabai -m window --resize bottom:0:-50; yabai -m window --resize top:0:-50
        lalt + cmd - j : yabai -m window --resize bottom:0:50; yabai -m window --resize top:0:50
        # Balance All Windows
        lalt + cmd - e : yabai -m space --balance
        # Send to Space
        shift + lctrl - 1 : yabai -m window --space 1
        shift + lctrl - 2 : yabai -m window --space 2
        shift + lctrl - 3 : yabai -m window --space 3
        shift + lctrl - 4 : yabai -m window --space 4
        shift + lctrl - 5 : yabai -m window --space 5
        '';
    };
  };
}
