{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.common.fish;
in {
  options.common.fish = {
    enable = lib.mkEnableOption "Enable personalised fish config";
  };

  config = lib.mkIf cfg.enable {
    programs.fish = {
      enable = true;

      interactiveShellInit = ''
        # vi key bindings
        set -g fish_key_bindings fish_vi_key_bindings
        set -g fish_greeting

        # vi mode cursor shapes
        set -g fish_cursor_default block
        set -g fish_cursor_insert line
        set -g fish_cursor_replace_one underscore
        set -g fish_cursor_visual block

        set -gx LESS "--chop-long-lines --HILITE-UNREAD --ignore-case --incsearch --jump-target=4 --LONG-PROMPT --no-init --quit-if-one-screen --RAW-CONTROL-CHARS --use-color --window=4"
        set -gx FZF_COMPLETION_DIR_COMMANDS "cd pushd rmdir tree ls"

        # TokyoNight Night theme
        set -g fish_color_normal c0caf5
        set -g fish_color_command 7dcfff
        set -g fish_color_keyword bb9af7
        set -g fish_color_quote e0af68
        set -g fish_color_redirection c0caf5
        set -g fish_color_end ff9e64
        set -g fish_color_option bb9af7
        set -g fish_color_error f7768e
        set -g fish_color_param 9d7cd8
        set -g fish_color_comment 565f89
        set -g fish_color_selection --background=283457
        set -g fish_color_search_match --background=283457
        set -g fish_color_operator 9ece6a
        set -g fish_color_escape bb9af7
        set -g fish_color_autosuggestion 565f89
        set -g fish_pager_color_progress 565f89
        set -g fish_pager_color_prefix 7dcfff
        set -g fish_pager_color_completion c0caf5
        set -g fish_pager_color_description 565f89
        set -g fish_pager_color_selected_background --background=283457
      '';

      plugins = [
        {
          name = "autopair";
          src = pkgs.fishPlugins.autopair-fish.src;
        }
        {
          name = "done";
          src = pkgs.fishPlugins.done.src;
        }
        {
          name = "sponge";
          src = pkgs.fishPlugins.sponge.src;
        }
        {
          name = "fzf-fish";
          src = pkgs.fishPlugins.fzf-fish.src;
        }
      ];
    };

    # enable fish integration for tools configured in other modules
    programs.direnv.enableFishIntegration = true;
    programs.starship.enableFishIntegration = true;
    programs.zoxide.enableFishIntegration = true;
    programs.fzf.enableFishIntegration = true;
  };
}
