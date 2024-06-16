{
  lib,
  config,
  ...
}: let
  cfg = config.common.matrix;
in {
  options.common.matrix = {
    enable = lib.mkEnableOption "Enable Matrix Synapse server";
    domain = lib.mkOption {
      type = lib.types.str;
      default = "matrix.pve.elmurphy.com";
      description = "Domain for Matrix server";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8448;
      description = "Port for Gitea";
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
      # Manual intervent is needed for db (see note above postgresql) and user creation
      # https://nixos.org/manual/nixos/stable/index.html#module-services-matrix-register-users
      matrix-synapse = {
        enable = true;
        settings = {
          server_name = "elmurphy.com";
          public_baseurl = "https://${cfg.domain}";
          listeners = [
            {
              port = cfg.port;
              bind_addresses = ["::1" "127.0.0.1"];
              type = "http";
              tls = false;
              x_forwarded = true;
              resources = [
                {
                  names = ["client"];
                  compress = true;
                }
                {
                  names = ["federation"];
                  compress = false;
                }
              ];
            }
          ];
          app_service_config_files = [
            # configuration details
            # https://github.com/NixOS/nixpkgs/pull/253196/commits/9e0457115e7eb3f106b9ea60ab3ca92daed5b03f
            # https://nixos.wiki/wiki/Matrix#mautrix-telegram
            "/var/lib/mautrix-whatsapp/whatsapp-registration.yaml"
          ];
          extraConfigFiles = [
            config.age.secrets.matrixSharedSecret.path
          ];
        };
      };
      mautrix-whatsapp = {
        enable = true;
        settings = {
          # option documentation
          # https://github.com/mautrix/whatsapp/blob/main/example-config.yaml
          appservice = {
            database = {
              type = "postgres";
              uri = "postgresql:///mautrix-whatsapp?host=/run/postgresql";
            };
          };
          bridge = {
            encryption = {
              allow = true;
              default = true;
              require = true;
            };
            permissions = {
              "@mm:elmurphy.com" = "admin";
            };
            history_sync = {
              request_full_sync = true;
            };
            mute_bridging = true;
          };
        };
      };
      postgresql = {
        enable = true;
        # Manually setup db via sudo -u postgres psql and executed linked sql
        # https://github.com/NixOS/nixpkgs/commit/52432f0a45216c0ac4aa4b0152b665fdf90b3029
        ensureDatabases = [
          "matrix-synapse"
          "mautrix-whatsapp"
        ];
        ensureUsers = [
          {
            name = "matrix-synapse";
            ensureDBOwnership = true;
          }
          {
            name = "mautrix-whatsapp";
            ensureDBOwnership = true;
          }
        ];
      };
      nginx = lib.mkIf cfg.nginx {
        virtualHosts."${cfg.domain}" = {
          forceSSL = true;
          useACMEHost = "elmurphy.com";
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.port}";
          };
        };
      };
    };

    # allow access to whatsapp-registration.yaml
    users.users.matrix-synapse.extraGroups = ["mautrix-whatsapp"];

    age.secrets.matrixSharedSecret = {
      file = ../../secrets/matrixSharedSecret.age;
      owner = "matrix-synapse";
    };
  };
}
