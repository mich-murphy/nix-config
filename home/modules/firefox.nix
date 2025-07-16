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
          privateDefault = "DuckDuckGo";
          force = true;
        };
        extensions = with pkgs.nur.repos.rycee.firefox-addons; [
          ublock-origin
          onepassword-password-manager
          new-tab-override
        ];
        settings = {
          "browser.startup.homepage" = "https://mich-murphy.github.io/startpage-bento/";
          "extensions.pocket.enabled" = false;
        };
      };
    };
  };
}
