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
    # bootstrap only: karabiner owns and rewrites its config (normalisation,
    # migrations, device entries), so enforcing the repo copy would fight it —
    # seed the file on first activation, then leave it alone
    home.activation.karabiner = lib.hm.dag.entryAfter ["writeBoundary"] ''
      if [ ! -e "${config.xdg.configHome}/karabiner/karabiner.json" ]; then
        run mkdir -p "${config.xdg.configHome}/karabiner"
        run cp "${karabinerConfig}" "${config.xdg.configHome}/karabiner/karabiner.json"
        run chmod u+rw "${config.xdg.configHome}/karabiner/karabiner.json"
      fi
    '';
  };
}
