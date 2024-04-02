{
  lib,
  config,
  ...
}:
with lib; let
  cfg = config.common.fzf;
in {
  options.common.fzf = {
    enable = mkEnableOption "Enable personalised FZF configuration";
  };

  config = mkIf cfg.enable {
    programs.fzf = {
      enable = true;
      enableZshIntegration = true;
      tmux.enableShellIntegration = true;
      defaultCommand = ''rg --files --hidden --glob "!.git"'';
      fileWidgetCommand = "$FZF_DEFAULT_COMMAND";
      colors = {
        "bg+" = "#1a1b26";
        fg = "#a9b1d6";
        "fg+" = "#c0caf5";
        border = "#1a1b26";
        spinner = "#3b4261";
        hl = "#7dcfff";
        header = "#e0af68";
        info = "#7aa2f7";
        pointer = "#7aa2f7";
        marker = "#f7768e";
        prompt = "#a9b1d6";
        "hl+" = "#7aa2f7";
      };
      defaultOptions = [
        "--height 60%"
        "--border none"
        "--layout reverse"
        "--color '$FZF_COLORS'"
        "--prompt '∷ '"
        "--pointer ▶"
        "--marker ⇒"
      ];
      fileWidgetOptions = [
        "--height 60%"
        "--border none"
        "--no-scrollbar"
        "--inline-info"
        "--layout reverse"
        "--color '$FZF_COLORS'"
        "--prompt '∷ '"
        "--pointer ▶"
        "--marker ⇒"
        "--preview 'bat --color=always {}'"
        "--preview-window '~2',border-none"
      ];
      changeDirWidgetOptions = [
        "--preview 'tree -C {} | head -n 10'"
      ];
    };
  };
}
