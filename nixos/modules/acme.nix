{
  lib,
  config,
  ...
}: let
  cfg = config.common.acme;
in {
  options.common.acme = {
    enable = lib.mkEnableOption "Enable Nginx with ACME wildcard certificate";
    email = lib.mkOption {
      type = lib.types.str;
      default = "acme@elmurphy.com";
      description = "Email address to use for ACME registration";
    };
    dnsProvider = lib.mkOption {
      type = lib.types.str;
      default = "cloudflare";
      description = "DNS provider used for ACME certificate";
    };
    domain = lib.mkOption {
      type = lib.types.str;
      default = "elmurphy.com";
      description = "Domain for certificate to be generated";
    };
  };

  config = lib.mkIf cfg.enable {
    services.nginx = {
      enable = true;
      recommendedGzipSettings = true;
      recommendedOptimisation = true;
      recommendedProxySettings = true;
      recommendedTlsSettings = true;
    };

    security.acme = {
      acceptTerms = true;
      preliminarySelfsigned = false;
      defaults = {
        email = cfg.email;
        dnsProvider = cfg.dnsProvider;
        credentialsFile = config.age.secrets.acmeCredentials.path;
      };
      certs.${cfg.domain} = {
        # domain = "*." + cfg.domain;
        extraDomainNames = [
          "*.elmurphy.com"
          "*.pve.elmurphy.com"
        ];
      };
    };

    users.users.nginx.extraGroups = ["acme"];

    age.secrets.acmeCredentials.file = ../../secrets/acmeCredentials.age;
  };
}
