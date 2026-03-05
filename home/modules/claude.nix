{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.claude;
  settingsFormat = pkgs.formats.json {};
in {
  options.common.claude = {
    enable = lib.mkEnableOption "Claude Code configuration";

    settings = lib.mkOption {
      type = settingsFormat.type;
      default = {};
      description = "Settings to write to ~/.claude/settings.json";
    };

    statusLine = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Contents of the statusline shell script (~/.claude/statusline.sh)";
    };
  };

  config = lib.mkIf cfg.enable {
    home.file.".claude/settings.json".source =
      settingsFormat.generate "claude-settings.json" cfg.settings;

    home.file.".claude/statusline.sh" = lib.mkIf (cfg.statusLine != "") {
      executable = true;
      text = cfg.statusLine;
    };
  };
}
