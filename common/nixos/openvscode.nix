{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.openvscode;
in {
  options.common.openvscode = {
    enable = mkEnableOption "Enable Open VSCode";
    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port for Open VSCode to be advertised on";
    };
    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Host address for Open VSCode";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      openvscode-server = {
        enable = true;
        inherit (cfg) host;
        inherit (cfg) port;
        extraPackages = with pkgs; [
          neovim
          python311
          python311Packages.pip
        ];
        telemetryLevel = "off";
        withoutConnectionToken = true;
      };
      nginx = mkIf cfg.nginx {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."code.pve.elmurphy.com" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://${cfg.host}:${toString cfg.port}";
            proxyWebsockets = true;
          };
        };
      };
    };

    nixpkgs.config.permittedInsecurePackages = [
      "nodejs-16.20.1"
    ];
  };
}
