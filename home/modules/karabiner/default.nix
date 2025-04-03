{
  lib,
  config,
  ...
}: let
  cfg = config.common.karabiner;
in {
  options.common.karabiner = {
    enable = lib.mkEnableOption "Add Karabiner configuration to .config/";
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."karabiner" = {
      enable = true;
      recursive = true;
      source = ./config;
      force = true;
    };
  };
}
