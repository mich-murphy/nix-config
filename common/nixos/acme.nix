{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.acme;
in {
  options.common.acme = {
    enable = mkEnableOption "Enable Nginx with ACME wildcard certificate";
  };

  config = mkIf cfg.enable {
    services.nginx.enable = true;

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

    users.users.nginx.extraGroups = ["acme"];

    age.secrets.acmeCredentials.file = ../../secrets/acmeCredentials.age;
  };
}
