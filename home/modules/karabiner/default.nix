{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.karabiner;
in {
  options.common.karabiner = {
    enable = mkEnableOption "Add Karabiner configuration to .config/";
  };

  config = mkIf cfg.enable {
    xdg.configFile."karabiner" = {
      enable = true;
      recursive = true;
      source = ./config;
    };
  };
}
