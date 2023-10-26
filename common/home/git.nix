{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.git;
in {
  options.common.git = {
    enable = mkEnableOption "Enable Git with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.git = {
      enable = true;
      userName = "mich-murphy";
      userEmail = "github@elmurphy.com";
      extraConfig = {
        init.defaultBranch = "main";
        pull.rebase = true;
      };
      diff-so-fancy.enable = true;
    };
  };
}
