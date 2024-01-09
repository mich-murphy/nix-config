{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.firefox-sync;
in {
  options.common.komga = {
    enable = mkEnableOption "Enable Firefox Sync";
    port = mkOption {
      type = types.port;
      default = 5000;
      description = "Port for Firefox Sync to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      firefox-syncserver = {
        enable = true;
        settings.port = cfg.port;
        singleNode = {
          enable = true;
          hostname = "firefox-sync.pve.elmurphy.com";
          enableTLS = true;
          enableNginx = cfg.nginx;
        };
      };
    };
  };
}
