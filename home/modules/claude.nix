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

    globalInstructions = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Global instructions to write to ~/.claude/CLAUDE.md";
    };

    agents = lib.mkOption {
      type = lib.types.attrsOf lib.types.lines;
      default = {};
      description = "Global agent files to write to ~/.claude/agents/<name>.md";
    };
  };

  config = lib.mkIf cfg.enable {
    home.file =
      {
        ".claude/settings.json".source =
          settingsFormat.generate "claude-settings.json" cfg.settings;
      }
      // lib.optionalAttrs (cfg.statusLine != "") {
        ".claude/statusline.sh" = {
          executable = true;
          text = cfg.statusLine;
        };
      }
      // lib.optionalAttrs (cfg.globalInstructions != "") {
        ".claude/CLAUDE.md" = {
          text = cfg.globalInstructions;
        };
      }
      // lib.mapAttrs' (name: content:
        lib.nameValuePair ".claude/agents/${name}.md" {text = content;})
      cfg.agents;
  };
}
