{
  lib,
  config,
  ...
}: let
  cfg = config.common.yazi;
in {
  options.common.yazi = {
    enable = lib.mkEnableOption "Enable Yazi with personalised settings";
  };

  config = lib.mkIf cfg.enable {
    programs.yazi = {
      enable = true;
      enableZshIntegration = true;
      settings = {
        log = {
          enabled = false;
        };
        manager = {
          show_hidden = true;
          sort_by = "mtime";
          sort_dir_first = true;
        };
      };
      theme = {
        status = {
          separator = {
            opening = "";
            closing = "";
          };
        };
      };
    };
  };
}
