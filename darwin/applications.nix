{pkgs, ...}: let
  plannotator = pkgs.stdenvNoCC.mkDerivation {
    pname = "plannotator";
    version = "0.24.1";
    src = pkgs.fetchurl {
      url = "https://github.com/backnotprop/plannotator/releases/download/v0.24.1/plannotator-darwin-arm64";
      hash = "sha256-FzObDbw4fXLIMzeifzmyOQfNK90jYd5TXHpaYzjwzZE=";
    };
    dontUnpack = true;
    dontStrip = true;
    installPhase = ''
      runHook preInstall
      install -Dm755 "$src" "$out/bin/plannotator"
      runHook postInstall
    '';
  };

  owncloudClient = pkgs.owncloud-client.overrideAttrs (old: {
    # Desktop 7 dropped support for ownCloud Classic. Nixpkgs provides 6.0.3,
    # but its Darwin build requires Sparkle even though auto-updates are off,
    # and its optional Finder extension cannot be built in the Nix sandbox.
    postPatch =
      (old.postPatch or "")
      + ''
        substituteInPlace CMakeLists.txt \
          --replace-fail "find_package(Sparkle REQUIRED)" "find_package(Sparkle)"
      '';
    cmakeFlags = (old.cmakeFlags or []) ++ ["-DBUILD_SHELL_INTEGRATION=OFF"];
    postFixup =
      (old.postFixup or "")
      + ''
        # qtWrapperArgs also wraps executable plugin bundles on Darwin. Restore
        # the real Mach-O bundles so QPluginLoader can load the VFS plugins.
        for plugin in "$out/Applications/KDE/owncloud.app/Contents/PlugIns/"*.so; do
          wrapped="$(dirname "$plugin")/.$(basename "$plugin")-wrapped"
          if [ -f "$wrapped" ]; then
            mv -f "$wrapped" "$plugin"
          fi
        done
      '';
  });
in {
  environment.variables.HOMEBREW_NO_ENV_HINTS = "1";

  environment.systemPackages = [
    pkgs.xcode-install
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs._1password-cli
    pkgs.gnused
    owncloudClient
    plannotator
  ];

  fonts.packages = [
    pkgs.nerd-fonts.jetbrains-mono
    pkgs.nerd-fonts._0xproto
  ];

  homebrew = {
    enable = true;
    enableFishIntegration = true;
    onActivation = {
      autoUpdate = true;
      upgrade = true;
      cleanup = "zap";
    };
    casks = [
      "claude-code@latest"
      "codex"
      "ghostty"
      "karabiner-elements"
      "wezterm"
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
      "slack"
      "microsoft-excel"
      "microsoft-powerpoint"
      "microsoft-word"
      "microsoft-teams"
      "docker-desktop"
      "displaylink"
      "raycast"
      "winbox"
      "xcodes-app"
      "linearmouse"
      "wispr-flow"
    ];
    brews = [
      "herdr"
      "pi-coding-agent"
      "mas"
      "mole"
      "xcodes"
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466;
      "Supernote Partner" = 1494992020;
    };
  };
}
