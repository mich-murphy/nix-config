{
  lib,
  config,
  ...
}: let
  cfg = config.common.murmur;
in {
  options.common.murmur = {
    enable = lib.mkEnableOption "Enable Murmur Server for Mumble";
    serverName = lib.mkOption {
      type = lib.types.str;
      default = "J&E Gaming";
      description = "Mumble server name";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 64738;
      description = "Port for Murmur";
    };
    nginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable nginx reverse proxy with SSL";
    };
  };

  config = lib.mkIf cfg.enable {
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

    age.secrets.murmurPass.file = ../../../secrets/murmurPass.age;
  };
}
