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
    port = mkOption {
      type = types.port;
      default = 64738;
      description = "Port for Murmur to be advertised on";
    };
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      murmur = {
        enable = true;
        bonjour = true;
        port = cfg.port;
        registerName = "J&E Gaming";
        environmentFile = config.age.secrets.murmurPass.path;
        password = "$MURMURD_PASSWORD";
      };
    };

    age.secrets.murmurPass.file = ../../secrets/murmurPass.age;
  };
}
