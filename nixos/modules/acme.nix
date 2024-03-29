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
    email = mkOption {
      type = types.str;
      default = "acme@elmurphy.com";
      description = "Email address to use for ACME registration";
    };
    dnsProvider = mkOption {
      type = types.str;
      default = "cloudflare";
      description = "DNS provider used for ACME certificate";
    };
    domain = mkOption {
      type = types.str;
      default = "elmurphy.com";
      description = "Domain for certificate to be generated";
    };
  };

  config = mkIf cfg.enable {
    services.nginx.enable = true;

    security.acme = {
      acceptTerms = true;
      preliminarySelfsigned = false;
      defaults = {
        email = cfg.email;
        dnsProvider = cfg.dnsProvider;
        credentialsFile = config.age.secrets.acmeCredentials.path;
      };
      certs.${cfg.domain} = {
        domain = "*." + cfg.domain;
      };
    };

    users.users.nginx.extraGroups = ["acme"];

    age.secrets.acmeCredentials.file = ../../secrets/acmeCredentials.age;
  };
}
