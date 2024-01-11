{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.wallabag;
in {
  options.common.wallabag = {
    enable = mkEnableOption "Enable Wallabag";
    port = mkOption {
      type = types.port;
      default = 7040;
      description = "Port for Wallabag to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    virtualisation.oci-containers = {
      backend = "docker";
      containers."wallabag" = {
        autoStart = true;
        image = "wallabag/wallabag";
        environment = {
          SYMFONY__ENV__DOMAIN_NAME = "https://wallabag.pve.elmurphy.com";
        };
        ports = ["${toString cfg.port}:80"];
        # https://github.com/wallabag/docker/issues/316#issuecomment-1465048886
        volumes = [
          "/data/wallabag/data:/var/www/wallabag/data"
          "/data/wallabag/images:/var/www/wallabag/web/assets/images"
        ];
      };
    };

    services.nginx = mkIf cfg.nginx {
      enable = true;
      recommendedGzipSettings = true;
      recommendedOptimisation = true;
      recommendedProxySettings = true;
      recommendedTlsSettings = true;
      virtualHosts."wallabag.pve.elmurphy.com" = {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString cfg.port}";
          proxyWebsockets = true;
        };
      };
    };
  };
}
