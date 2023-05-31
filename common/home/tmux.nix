{ lib, config, pkgs, stdenv, ... }:

with lib;

let
  cfg = config.common.tmux;
in
{
  options.common.tmux = {
    enable = mkEnableOption "Enable Tmux with personalised settings";
  };

  config = mkIf cfg.enable {
    programs.tmux = {
      enable = true;
      baseIndex = 1;
      disableConfirmationPrompt = true;
      escapeTime = 0;
      historyLimit = 10000;
      keyMode = "vi";
      newSession = false;
      prefix = "C-Space";
      terminal = "screen-256color";
      tmuxp.enable = true;
      plugins = with pkgs; [
        tmuxPlugins.tmux-fzf
        tmuxPlugins.vim-tmux-navigator
        {
          plugin = tmuxPlugins.power-theme;
          extraConfig = "set -g @tmux_power_theme '#7aa2f7'";
        }
      ];
      extraConfig = ''
        # Additional terminal color settings
        set-option -ga terminal-overrides ",xterm-256color:Tc"

        # Vim settings
        set-option -g focus-events on

        # Easier split pane bindings
        bind - split-window -v -c "#{pane_current_path}"
        bind | split-window -h -c "#{pane_current_path}"

        # Vim keybindings
        unbind -T copy-mode-vi Space;
        unbind -T copy-mode-vi Enter;
        bind -T copy-mode-vi v send-keys -X begin-selection
        bind -T copy-mode-vi y send-keys -X copy-pipe-and-cancel "xsel --clipboard" 

        # Allow mouse scrolling
        set -g mouse on
      '';
    };
  };
}
