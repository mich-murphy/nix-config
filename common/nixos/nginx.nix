{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.nginx;
in
{
  options.common.nginx = {
    enable = mkEnableOption "Enable Nginx with ACME wildcard certificate";
  };

  config = mkIf cfg.enable {
    services.nginx = {
      enable = true;
      recommendedGzipSettings = true;
      recommendedOptimisation = true;
      recommendedProxySettings = true;
      recommendedTlsSettings = true;
      virtualHosts."navidrome.pve.elmurphy.com"= {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:4533";
          proxyWebsockets = true;
        };
      };
      virtualHosts."komga.pve.elmurphy.com"= {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:6080";
          proxyWebsockets = true;
        };
      };
      virtualHosts."uptime-kuma.pve.elmurphy.com"= {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:3001";
          proxyWebsockets = true;
        };
      };
    };

    security.acme = {
      acceptTerms = true;
      preliminarySelfsigned = false;
      defaults = {
        email = "acme@elmurphy.com";
        dnsProvider = "cloudflare";
        credentialsFile = config.age.secrets.acmeCredentials.path;
      };
      certs."elmurphy.com" = {
        domain = "*.elmurphy.com";
      };
    };

    users.users.nginx.extraGroups = [ "acme" ];

    age.secrets.acmeCredentials.file = ../../secrets/acmeCredentials.age;
  };
}
