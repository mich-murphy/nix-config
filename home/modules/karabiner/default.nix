{
  lib,
  config,
  ...
}: let
  cfg = config.common.karabiner;
  karabinerConfig = ./config/karabiner.json;
in {
  options.common.karabiner = {
    enable = lib.mkEnableOption "Add Karabiner configuration to .config/";
  };

  config = lib.mkIf cfg.enable {
    home.activation.karabiner = lib.hm.dag.entryAfter ["writeBoundary"] ''
      run mkdir -p "${config.xdg.configHome}/karabiner"
      run cp -f "${karabinerConfig}" "${config.xdg.configHome}/karabiner/karabiner.json"
      run chmod u+rw "${config.xdg.configHome}/karabiner/karabiner.json"
    '';
  };
}
