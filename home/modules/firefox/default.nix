{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.firefox;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in {
  options.common.firefox = {
    enable = mkEnableOption "Enable Firefox with custom settings and userChrome.css";
  };

  config = mkIf cfg.enable {
    programs.firefox = {
      enable = true;
      package = fakepkg "firefox";
      profiles."mm" = {
        isDefault = true;
        search = {
          default = "DuckDuckGo";
          force = true;
        };
        extensions = with pkgs.nur.repos.rycee.firefox-addons; [
          ublock-origin
          skip-redirect
          onepassword-password-manager
          new-tab-override
        ];
        settings = {
          # enable loading of custom userchrome
          "toolkit.legacyUserProfileCustomizations.stylesheets" = true;
        };
        # load customised theme css
        userChrome = builtins.readFile ./userChrome.css;
      };
    };
  };
}
