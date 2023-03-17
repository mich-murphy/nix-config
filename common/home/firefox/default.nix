{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.firefox;
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
in
{
  options.common.firefox = {
    enable = mkEnableOption "Enable Firefox with custom settings and userChrome.css";
  };

  config = mkIf cfg.enable {
    programs.firefox = {
      enable = true;
      # fake package - managed by homebrew instead
      package = fakepkg "firefox";
      profiles."mm" = {
        isDefault = true;
        search = {
          default = "DuckDuckGo";
          force = true;
        };
        extensions = with pkgs.nur.repos.rycee.firefox-addons; [
          ublock-origin
          onepassword-password-manager
          decentraleyes
          privacy-badger
          new-tab-override
        ];
        settings = {
          # https://sunknudsen.com/privacy-guides/how-to-configure-firefox-for-privacy-and-security
          # Firefox hardening using preferences (automated)
          "app.normandy.first_run" = false;
          "app.shield.optoutstudies.enabled" = false;
          "app.update.auto" = false;
          "browser.contentblocking.category" = "custom";
          "browser.download.useDownloadDir" = false;
          "browser.formfill.enable" = false;
          "browser.newtabpage.activity-stream.asrouter.userprefs.cfr.addons" = false;
          "browser.newtabpage.activity-stream.asrouter.userprefs.cfr.features" = false;
          "browser.newtabpage.activity-stream.feeds.section.topstories" = false;
          "browser.newtabpage.activity-stream.feeds.topsites" = false;
          "browser.search.suggest.enabled" = false;
          "browser.urlbar.placeholderName" = "DuckDuckGo";
          "datareporting.healthreport.uploadEnabled" = false;
          "doh-rollout.disable-heuristics" = true;
          "dom.forms.autocomplete.formautofill" = true;
          "dom.security.https_only_mode_ever_enabled" = true;
          "dom.security.https_only_mode" = true;
          "extensions.formautofill.addresses.enabled" = false;
          "extensions.formautofill.creditCards.enabled" = false;
          "extensions.pocket.enabled" = false;
          "identity.fxaccounts.enabled" = false;
          "layout.spellcheckDefault" = 1; # Used to disable spellchecker… set to `0` for increased privacy
          "media.peerconnection.enabled" = true; # Used to disable WebRTC (mitigating WebRTC leaks)… set to `true` to enable WebRTC
          "network.cookie.cookieBehavior" = 1;
          "network.cookie.lifetimePolicy" = 0; # Used to delete cookies when Firefox is closed… set to `0` to enable default cookie persistence
          "network.proxy.socks_remote_dns" = true;
          "network.trr.custom_uri" = "https:#doh.mullvad.net/dns-query";
          "network.trr.mode" = 5; # Used to enable Mullvad DNS over HTTPS… set to `5` to disable Mullvad DNS over HTTPS
          "network.trr.uri" = "https:#doh.mullvad.net/dns-query";
          "places.history.enabled" = false;
          "privacy.donottrackheader.enabled" = true;
          "privacy.history.custom" = true;
          "privacy.sanitize.sanitizeOnShutdown" = false; # Used to delete cookies and site data when Firefox is closed… set to `false` to enable cookie and site data persistence
          "privacy.trackingprotection.enabled" = true;
          "privacy.trackingprotection.socialtracking.enabled" = true;
          "signon.management.page.breach-alerts.enabled" = false;
          "signon.rememberSignons" = false;
          # Firefox hardening using about:config (arkenfox/user.js recommendations = automated)
          "accessibility.force_disabled" = 1;
          "app.normandy.api_url" = "";
          "app.normandy.enabled" = false;
          "beacon.enabled" = false;
          "browser.pagethumbnails.capturing_disabled" = true;
          "browser.ping-centre.telemetry" = false;
          "browser.places.speculativeConnect.enabled" = false;
          "browser.sessionstore.privacy_level" = 2;
          "browser.ssl_override_behavior" = 1;
          "browser.tabs.crashReporting.sendReport" = false;
          "browser.uitour.enabled" = false;
          "browser.uitour.url" = "";
          "browser.urlbar.speculativeConnect.enabled" = false;
          "browser.urlbar.suggest.quicksuggest.nonsponsored" = false;
          "browser.urlbar.suggest.quicksuggest.sponsored" = false;
          "browser.urlbar.trimURLs" = false;
          "browser.xul.error_pages.expert_bad_cert" = true;
          "captivedetect.canonicalURL" = "";
          "datareporting.policy.dataSubmissionEnabled" = false;
          "dom.security.https_only_mode_send_http_background_request" = false;
          "extensions.getAddons.showPane" = false;
          "extensions.htmlaboutaddons.recommendations.enabled" = false;
          "geo.provider.use_corelocation" = false;
          "network.auth.subresource-http-auth-allow" = 1;
          "network.captive-portal-service.enabled" = false;
          "network.connectivity-service.enabled" = false;
          "network.dns.disableIPv6" = true;
          "network.dns.disablePrefetch" = true;
          "network.http.speculative-parallel-limit" = 0;
          "network.predictor.enabled" = false;
          "network.prefetch-next" = false;
          "pdfjs.enableScripting" = false;
          "privacy.userContext.enabled" = true;
          "privacy.userContext.ui.enabled" = true;
          "security.cert_pinning.enforcement_level" = 2;
          "security.mixed_content.block_display_content" = true;
          "security.OCSP.require" = true;
          "security.pki.crlite_mode" = 2;
          "security.pki.sha1_enforcement_level" = 1;
          "security.remote_settings.crlite_filters.enabled" = true;
          "security.ssl.require_safe_negotiation" = true;
          "security.ssl.treat_unsafe_negotiation_as_broken" = true;
          "security.tls.enable_0rtt_data" = false;
          "toolkit.coverage.endpoint.base" = "";
          "toolkit.coverage.opt-out" = true;
          "toolkit.telemetry.coverage.opt-out" = true;
          # Firefox fingerprinting hardening using about:config (automated)
          "privacy.resistFingerprinting" = false; # Used to help resist fingerprinting but breaks dark mode and screenshots (among other features)… set to `true` for increased privacy
          "privacy.resistFingerprinting.block_mozAddonManager" = true;
          "privacy.resistFingerprinting.letterboxing" = false; # Used to help resist fingerprinting… set to `false` to disable letterboxing
          "webgl.disabled" = true;
        };
        userChrome = builtins.readFile ./userChrome.css;
      };
    };
  };
}
