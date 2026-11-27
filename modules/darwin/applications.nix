{ pkgs, ... }:
{
  environment.variables.HOMEBREW_NO_ENV_HINTS = "1";

  environment.systemPackages = [
    pkgs.tmux
    pkgs.mosh
    pkgs.nmap
    pkgs.gnused
    pkgs.moshi-hook.app
    pkgs.mas
  ];

  system.activationScripts.postActivation.text = ''
    /usr/libexec/ApplicationFirewall/socketfilterfw --add ${pkgs.mosh}/bin/mosh-server
    /usr/libexec/ApplicationFirewall/socketfilterfw --unblockapp ${pkgs.mosh}/bin/mosh-server
  '';

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
      "ghostty"
      "karabiner-elements"
      "whichspace"
      "1password"
      "stats"
      "obsidian"
      "zotero"
      "tailscale-app"
      "thaw@beta"
      "firefox"
      "google-chrome"
      "iina"
      "plexamp"
      "spotify"
      "slack"
      "orbstack"
      "raycast"
      "linearmouse"
      "wispr-flow"
      "nextcloud"
    ];
    masApps = {
      "Xcode" = 497799835;
      "Windows App" = 1295203466;
      "Supernote Partner" = 1494992020;
    };
  };
}
