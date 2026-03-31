{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.claude;
  settingsFormat = pkgs.formats.json {};
  agentDir = ./agents;
  agentFiles = lib.filterAttrs (n: v: v == "regular" && lib.hasSuffix ".md" n) (builtins.readDir agentDir);
in {
  options.common.claude = {
    enable = lib.mkEnableOption "Claude Code configuration";

    settings = lib.mkOption {
      type = settingsFormat.type;
      default = {
        "$schema" = "https://json.schemastore.org/claude-code-settings.json";
        model = "claude-opus-4-6";
        permissions = {
          allow = [
            "Read"
            "Glob"
            "Grep"
            "Edit"
            "Write"
            "Bash(nix fmt)"
            "Bash(nix flake*)"
            "Bash(nix eval*)"
            "Bash(nix search*)"
            "Bash(nix repl*)"
            "Bash(darwin-rebuild build --flake .)"
            "Bash(darwin-rebuild check --flake .)"
            "Bash(git status)"
            "Bash(git diff*)"
            "Bash(git log*)"
            "Bash(ls*)"
            "Bash(find*)"
            "Bash(mkdir*)"
            "Bash(brew info*)"
            "Bash(brew list*)"
            "Bash(brew search*)"
          ];
          deny = [
            "Bash(rm -rf*)"
            "Bash(darwin-rebuild switch*)"
            "Bash(nix-collect-garbage*)"
            "Bash(git push*)"
            "Bash(git reset*)"
            "Bash(brew uninstall*)"
            "Bash(brew install*)"
          ];
        };
        hooks = {
          Notification = [
            {
              matcher = "";
              hooks = [
                {
                  type = "command";
                  command = "osascript -e 'display notification \"Claude Code wants to harass you with needless questions\" with title \"Claude Code\"'";
                }
              ];
            }
          ];
          PostToolUse = [
            {
              matcher = "Edit|Write";
              hooks = [
                {
                  type = "command";
                  command = "if [[ \"$CLAUDE_TOOL_ARG_FILE_PATH\" == *.nix ]]; then nix fmt 2>/dev/null; fi";
                }
                {
                  type = "command";
                  command = "if [[ \"$CLAUDE_TOOL_ARG_FILE_PATH\" == *.tf ]] || [[ \"$CLAUDE_TOOL_ARG_FILE_PATH\" == *.tfvars ]]; then tofu fmt \"$CLAUDE_TOOL_ARG_FILE_PATH\" 2>/dev/null; fi";
                }
              ];
            }
          ];
        };
        spinnerVerbs = {
          mode = "replace";
          verbs = [
            "Fucking Shit Up"
            "One Big Cluster Fuck Coming Right Up"
            "Creating Some Quality AI Slop"
            "Taking A Big Shit On Your Codebase"
          ];
        };
        statusLine = {
          type = "command";
          command = "~/.claude/statusline.sh";
        };
      };
      description = "Settings to write to ~/.claude/settings.json";
    };

    statusLine = lib.mkOption {
      type = lib.types.lines;
      default = builtins.readFile ./statusline.sh;
      description = "Contents of the statusline shell script (~/.claude/statusline.sh)";
    };

    globalInstructions = lib.mkOption {
      type = lib.types.lines;
      default = builtins.readFile ./CLAUDE.md;
      description = "Global instructions to write to ~/.claude/CLAUDE.md";
    };

    agents = lib.mkOption {
      type = lib.types.attrsOf lib.types.lines;
      default = lib.mapAttrs' (name: _:
        lib.nameValuePair (lib.removeSuffix ".md" name) (builtins.readFile (agentDir + "/${name}")))
      agentFiles;
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
