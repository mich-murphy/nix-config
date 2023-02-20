{ lib, config, pkgs, ... }:

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
      customPaneNavigationAndResize = true;
      disableConfirmationPrompt = true;
      escapeTime = 10;
      historyLimit = 10000;
      keyMode = "vi";
      newSession = true;
      prefix = "C-Space";
      terminal = "screen-256color";
      tmuxp.enable = true;
      plugins = with pkgs; [
        tmuxPlugins.tmux-fzf
        {
          plugin = tmuxPlugins.power-theme;
          extraConfig = "set -g @tmux_power_theme 'moon'";
        }
      ];
      extraConfig = ''
        # Vim settings
        set-option -g focus-events on
        # Changing key bindings
        unbind r
        bind r source-file $XDG_CONFIG_HOME/tmux/tmux.conf \; display "Reloaded tmux conf"
        set -g mouse on
        unbind v
        unbind h
        unbind %
        unbind '"'
        bind v split-window -h -c "#{pane_current_path}"
        bind h split-window -v -c "#{pane_current_path}"
        bind -n C-h select-pane -L
        bind -n C-j select-pane -D
        bind -n C-k select-pane -U
        bind -n C-l select-pane -R
        unbind n
        unbind w
        bind n command-prompt "rename-window '%%'"
        bind w new-window -c "#{pane_current_path}"
        bind -n M-j previous-window
        bind -n M-k next-window
        # Vim keybindings
        unbind -T copy-mode-vi Space;
        unbind -T copy-mode-vi Enter;
        bind -T copy-mode-vi v send-keys -X begin-selection
        bind -T copy-mode-vi y send-keys -X copy-pipe-and-cancel "xsel --clipboard" 
        set -g -a terminal-overrides ',*:Ss=\E[%p1%d q:Se=\E[2 q'
        is_vim="ps -o state= -o comm= -t '#{pane_tty}' \
            | grep -iqE '^[^TXZ ]+ +(\\S+\\/)?g?(view|n?vim?x?)(diff)?$'"
        bind -n C-h if-shell "$is_vim" "send-keys C-h"  "select-pane -L"
        bind -n C-j if-shell "$is_vim" "send-keys C-j"  "select-pane -D"
        bind -n C-k if-shell "$is_vim" "send-keys C-k"  "select-pane -U"
        bind -n C-l if-shell "$is_vim" "send-keys C-l"  "select-pane -R"
        bind -n C-\\ if-shell "$is_vim" "send-keys C-\\" "select-pane -l"
      '';
    };
  };
}
