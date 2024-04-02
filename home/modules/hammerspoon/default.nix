{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.hammerspoon;
in {
  options.common.hammerspoon = {
    enable = mkEnableOption "Add hammerspoon configuration to home directory";
  };

  config = mkIf cfg.enable {
    home.file.".hammerspoon" = {
      enable = true;
      recursive = true;
      source = ./config;
    };
  };
}
