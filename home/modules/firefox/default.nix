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
          skip-redirect
          onepassword-password-manager
          new-tab-override
          omnivore
        ];
        settings = {
          "browser.startup.homepage" = "https://mich-murphy.github.io/startpage-bento/";
          "extensions.pocket.enabled" = false;
          # enable loading of custom userchrome
          "toolkit.legacyUserProfileCustomizations.stylesheets" = true;
          "layers.acceleration.force-enabled" = true;
          "gfx.webrender.all" = true;
          "svg.context-properties.content.enabled" = true;
        };
        # load customised theme css
        userChrome = builtins.readFile ./userChrome.css;
        userContent = ''
          /*
          ┌─┐┬┌┬┐┌─┐┬  ┌─┐
          └─┐││││├─┘│  ├┤
          └─┘┴┴ ┴┴  ┴─┘└─┘
          ┌─┐┌─┐─┐ ┬
          ├┤ │ │┌┴┬┘
          └  └─┘┴ └─

          by Miguel Avila

          */

          :root {
            scrollbar-width: none !important;
          }

          @-moz-document url(about:privatebrowsing) {
            :root {
              scrollbar-width: none !important;
            }
          }
        '';
      };
    };
  };
}
