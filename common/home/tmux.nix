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
      newSession = true;
      prefix = "C-Space";
      terminal = "screen-256color";
      tmuxp.enable = true;
      plugins = with pkgs; [
        {
          plugin = tmuxPlugins.power-theme;
          extraConfig = "set -g @tmux_power_theme 'moon'";
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

        # Smart pane switching with awareness of Vim splits.
        # See: https://github.com/christoomey/vim-tmux-navigator
        is_vim="ps -o state= -o comm= -t '#{pane_tty}' \
            | grep -iqE '^[^TXZ ]+ +(\\S+\\/)?g?(view|l?n?vim?x?)(diff)?$'"
        bind-key -n 'C-h' if-shell "$is_vim" 'send-keys C-h'  'select-pane -L'
        bind-key -n 'C-j' if-shell "$is_vim" 'send-keys C-j'  'select-pane -D'
        bind-key -n 'C-k' if-shell "$is_vim" 'send-keys C-k'  'select-pane -U'
        bind-key -n 'C-l' if-shell "$is_vim" 'send-keys C-l'  'select-pane -R'
        tmux_version='$(tmux -V | sed -En "s/^tmux ([0-9]+(.[0-9]+)?).*/\1/p")'
        if-shell -b '[ "$(echo "$tmux_version < 3.0" | bc)" = 1 ]' \
            "bind-key -n 'C-\\' if-shell \"$is_vim\" 'send-keys C-\\'  'select-pane -l'"
        if-shell -b '[ "$(echo "$tmux_version >= 3.0" | bc)" = 1 ]' \
            "bind-key -n 'C-\\' if-shell \"$is_vim\" 'send-keys C-\\\\'  'select-pane -l'"

        bind-key -T copy-mode-vi 'C-h' select-pane -L
        bind-key -T copy-mode-vi 'C-j' select-pane -D
        bind-key -T copy-mode-vi 'C-k' select-pane -U
        bind-key -T copy-mode-vi 'C-l' select-pane -R
        bind-key -T copy-mode-vi 'C-\' select-pane -l
      '';
    };
  };
}
