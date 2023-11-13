{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.calibre-web;
in {
  options.common.calibre-web = {
    enable = mkEnableOption "Enable Calibre-Web";
    port = mkOption {
      type = types.port;
      default = 8083;
      description = "Port for Calibe-Web to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      calibre-web = {
        enable = true;
        listen.ip = "127.0.0.1";
        options = {
          calibreLibrary = "/data/media/books";
          enableBookUploading = true;
        };
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."calibre.pve.elmurphy.com" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.port}";
            proxyWebsockets = true;
            extraConfig = ''
              proxy_set_header X-Script-Name /calibre-web;
              client_max_body_size 1024M;
            '';
          };
        };
      };
    };

    users.users.calibre-web.extraGroups = ["media"];
  };
}
