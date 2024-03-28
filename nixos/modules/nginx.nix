{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.nginx;
in {
  options.common.nginx = {
    enable = mkEnableOption "Enable Nginx";
  };

  config = mkIf cfg.enable {
    services = {
      nginx = {
        enable = true;
        recommendedGzipSettings = true;
        recommendedOptimisation = true;
        recommendedProxySettings = true;
        recommendedTlsSettings = true;
        virtualHosts."git.pve.elmurphy.com" = {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://100.65.11.154:3001";
          };
        };
      };
    };
  };
}
