{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.plex;
  # enable plugin for audiobook metadata
  audnexusPlugin = pkgs.stdenv.mkDerivation {
    name = "Audnexus.bundle";
    src = pkgs.fetchurl {
      url = "https://github.com/djdembeck/Audnexus.bundle/archive/refs/tags/v1.3.1.zip";
      sha256 = "sha256-P6lQ4gR+a4CJ2nJGSrA2HcWx9/KtlA9/SLMTxrreckA=";
    };
    buildInputs = [pkgs.unzip];
    installPhase = "mkdir -p $out; cp -R * $out/";
  };
in {
  options.common.plex = {
    enable = mkEnableOption "Enable Plex with Audnexus plugin for audiobooks";
    extraGroups = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Additional groups for plex user";
      example = ["media"];
    };
    enableAudnexus = mkOption {
      type = types.bool;
      default = false;
      description = "Enable audiobook metadata plugin";
    };
    enableTautulli = mkOption {
      type = types.bool;
      default = false;
      description = "Enable Tautulli";
    };
    tautulliDomain = mkOption {
      type = types.str;
      default = "tautulli.pve.elmurphy.com";
      description = "Domain for Tautulli";
    };
    tautulliHostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address for Tautulli host";
    };
    enableOverseerr = mkOption {
      type = types.bool;
      default = false;
      description = "Enable Overseerr";
    };
    overseerrDomain = mkOption {
      type = types.str;
      default = "overseerr.pve.elmurphy.com";
      description = "Domain for Tautulli";
    };
    overseerrHostAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "IP address for Overseer host";
    };
    overseerrPort = mkOption {
      type = types.port;
      default = 5055;
      description = "Port for Overseerr";
    };
    domain = mkOption {
      type = types.str;
      default = "plex.pve.elmurphy.com";
      description = "Domain for Plex";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nginx -> config.services.nginx.enable == true;
        message = "Nginx needs to be enabled";
      }
    ];

    virtualisation.oci-containers = mkIf cfg.enableOverseerr {
      backend = "docker";
      containers."overseerr" = {
        autoStart = true;
        image = "sctx/overseerr:latest";
        environment = {
          LOG_LEVEL = "debug";
          TZ = "Australia/Melbourne";
        };
        ports = ["${toString cfg.overseerrPort}:5055"];
        volumes = [
          "/var/lib/overseerr:/app/config"
        ];
        extraOptions = ["--network=host"];
      };
    };
    services = {
      plex = {
        enable = true;
        extraPlugins =
          if cfg.enableAudnexus
          then [audnexusPlugin]
          else [];
      };
      tautulli.enable =
        if cfg.enableTautulli
        then true
        else false;
      nginx = mkIf cfg.nginx {
        virtualHosts.${cfg.overseerrDomain} = mkIf cfg.enableOverseerr {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.overseerrHostAddress}:${toString cfg.overseerrPort}";
            proxyWebsockets = true;
          };
        };
        virtualHosts.${cfg.tautulliDomain} = mkIf config.services.tautulli.enable {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://${cfg.tautulliHostAddress}:8181";
          };
        };
        virtualHosts.${cfg.domain} = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          extraConfig = ''
            # some players don't reopen a socket and playback stops totally instead of resuming after an extended pause
            send_timeout 100m;

            # why this is important: https://blog.cloudflare.com/ocsp-stapling-how-cloudflare-just-made-ssl-30/
            ssl_stapling on;
            ssl_stapling_verify on;

            ssl_protocols TLSv1 TLSv1.1 TLSv1.2;
            ssl_prefer_server_ciphers on;
            # intentionally not hardened for security for player support and encryption video streams has a lot of overhead with something like AES-256-GCM-SHA384.
            ssl_ciphers 'ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES256-GCM-SHA384:DHE-RSA-AES128-GCM-SHA256:DHE-DSS-AES128-GCM-SHA256:kEDH+AESGCM:ECDHE-RSA-AES128-SHA256:ECDHE-ECDSA-AES128-SHA256:ECDHE-RSA-AES128-SHA:ECDHE-ECDSA-AES128-SHA:ECDHE-RSA-AES256-SHA384:ECDHE-ECDSA-AES256-SHA384:ECDHE-RSA-AES256-SHA:ECDHE-ECDSA-AES256-SHA:DHE-RSA-AES128-SHA256:DHE-RSA-AES128-SHA:DHE-DSS-AES128-SHA256:DHE-RSA-AES256-SHA256:DHE-DSS-AES256-SHA:DHE-RSA-AES256-SHA:ECDHE-RSA-DES-CBC3-SHA:ECDHE-ECDSA-DES-CBC3-SHA:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA256:AES256-SHA256:AES128-SHA:AES256-SHA:AES:CAMELLIA:DES-CBC3-SHA:!aNULL:!eNULL:!EXPORT:!DES:!RC4:!MD5:!PSK:!aECDH:!EDH-DSS-DES-CBC3-SHA:!EDH-RSA-DES-CBC3-SHA:!KRB5-DES-CBC3-SHA';

            # forward real ip and host to Plex
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header Host $server_addr;
            proxy_set_header Referer $server_addr;
            proxy_set_header Origin $server_addr;

            # plex has A LOT of javascript, xml and html. this helps a lot, but if it causes playback issues with devices turn it off.
            gzip on;
            gzip_vary on;
            gzip_min_length 1000;
            gzip_proxied any;
            gzip_types text/plain text/css text/xml application/xml text/javascript application/x-javascript image/svg+xml;
            gzip_disable "MSIE [1-6]\.";

            # nginx default client_max_body_size is 1MB, which breaks camera upload feature from the phones.
            # increasing the limit fixes the issue. Anyhow, if 4K videos are expected to be uploaded, the size might need to be increased even more
            client_max_body_size 100M;

            # plex headers
            proxy_set_header X-Plex-Client-Identifier $http_x_plex_client_identifier;
            proxy_set_header X-Plex-Device $http_x_plex_device;
            proxy_set_header X-Plex-Device-Name $http_x_plex_device_name;
            proxy_set_header X-Plex-Platform $http_x_plex_platform;
            proxy_set_header X-Plex-Platform-Version $http_x_plex_platform_version;
            proxy_set_header X-Plex-Product $http_x_plex_product;
            proxy_set_header X-Plex-Token $http_x_plex_token;
            proxy_set_header X-Plex-Version $http_x_plex_version;
            proxy_set_header X-Plex-Nocache $http_x_plex_nocache;
            proxy_set_header X-Plex-Provides $http_x_plex_provides;
            proxy_set_header X-Plex-Device-Vendor $http_x_plex_device_vendor;
            proxy_set_header X-Plex-Model $http_x_plex_model;

            # websockets
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";

            # buffering off send to the client as soon as the data is received from Plex.
            proxy_redirect off;
            proxy_buffering off;
          '';
          locations."/" = {
            proxyPass = "http://${cfg.domain}:32400";
          };
        };
      };
    };

    users.users.plex.extraGroups = cfg.extraGroups;
  };
}
