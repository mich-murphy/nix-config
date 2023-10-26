{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.roon-server;
in {
  options.common.roon-server = {
    enable = mkEnableOption "Enable Roon Server";
  };

  config = mkIf cfg.enable {
    services.roon-server.enable = true;

    users.users.roon-server.extraGroups = ["media"];
  };
}
