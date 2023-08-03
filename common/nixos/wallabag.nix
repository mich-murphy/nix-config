{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.wallabag;
in
{
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
		  SYMFONY__ENV__DOMAIN_NAME = "http://127.0.0.1";
		  SYMFONY__ENV__DATABASE_DRIVER = "pdo_sqlite";
        };
        ports = [ "${toString cfg.port}:80" ];
      };
    };

    services.nginx = mkIf cfg.nginx {
      virtualHosts."wallabag.pve.elmurphy.com"= {
        enableACME = true;
        addSSL = true;
        acmeRoot = null;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString cfg.port}";
          # proxyWebsockets = true;
          extraConfig = ''
            proxy_set_header X-Forwarded-Host $server_name;
            proxy_set_header X-Forwarded-Proto https;
            proxy_set_header X-Forwarded-For $remote_addr;
          '';
        };
      };
    };
  };
}
