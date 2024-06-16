{
  lib,
  config,
  ...
}: let
  cfg = config.common.hammerspoon;
in {
  options.common.hammerspoon = {
    enable = lib.mkEnableOption "Add hammerspoon configuration to home directory";
  };

  config = lib.mkIf cfg.enable {
    home.file.".hammerspoon" = {
      enable = true;
      recursive = true;
      source = ./config;
    };
  };
}
