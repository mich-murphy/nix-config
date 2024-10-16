{
  lib,
  config,
  ...
}: let
  cfg = config.common.stirling-pdf;
in {
  options.common.stirling-pdf = {
    enable = lib.mkEnableOption "Enable stirling-pdf";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "pdf.pve.elmurphy.com";
      description = "Domain for stirling-pdf";
    };
    hostAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "IP address of stirling-pdf host";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8001;
      description = "Port for stirling-pdf";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    services = {
      stirling-pdf = {
        enable = true;
        environment = {
          INSTALL_BOOK_AND_ADVANCED_HTML_OPS = "true";
          SERVER_PORT = cfg.port;
          LANGS = "en_GB";
        };
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.hostAddress}:${toString cfg.port}";
          };
        };
      };
    };
  };
}
