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
    # virtualisation.oci-containers = {
    #   backend = "docker";
    #   containers."calibre-web" = {
    #     autoStart = true;
    #     image = "lscr.io/linuxserver/calibre-web:latest";
    #     environment = {
    #       PUID = "1000";
    #       PGID = "1000";
    #       TZ = "Australia/Melbourne";
    #       DOCKER_MODS = "linuxserver/mods:universal-calibre";
    #     };
    #     ports = ["${toString cfg.port}:8083"];
    #     volumes = [
    #       "/var/lib/calibre-web:/config"
    #       "/data/media/books:/books"
    #     ];
    #   };
    # };

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
