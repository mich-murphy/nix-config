{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.firefox;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in {
  options.common.firefox = {
    enable = lib.mkEnableOption "Enable Firefox with custom settings and userChrome.css";
  };

  config = lib.mkIf cfg.enable {
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
          omnivore
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
