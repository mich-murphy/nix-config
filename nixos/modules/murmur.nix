{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.murmur;
in {
  options.common.murmur = {
    enable = mkEnableOption "Enable Murmur Server for Mumble";
    serverName = mkOption {
      type = types.str;
      default = "J&E Gaming";
      description = "Mumble server name";
    };
    port = mkOption {
      type = types.port;
      default = 64738;
      description = "Port for Murmur";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      murmur = {
        enable = true;
        bonjour = true;
        port = cfg.port;
        registerName = cfg.serverName;
        environmentFile = config.age.secrets.murmurPass.path;
        password = "$MURMURD_PASSWORD";
      };
    };

    age.secrets.murmurPass.file = ../../secrets/murmurPass.age;
  };
}
