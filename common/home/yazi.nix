{ lib, config, ... }:

with lib;

let
  cfg = config.common.yazi;
in
{
  options.common.yazi = {
    enable = mkEnableOption "Enable Yazi with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.yazi = {
      enable = true;
      enableZshIntegration = true;
      settings = {
        log = {
          enabled = false;
        };
        manager = {
          show_hidden = true;
          sort_by = "modified";
          sort_dir_first = true;
        };
      };
      theme = {
        status = {
          separator = { opening = ""; closing = ""; };
        };
      };
    };
  };
}
