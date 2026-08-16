{pkgs, ...}: let
  maintenance = pkgs.writeShellApplication {
    name = "ai-dev-maintenance";
    runtimeInputs = [
      pkgs.bash
      pkgs.coreutils
      pkgs.curl
      pkgs.gawk
      pkgs.gnugrep
      pkgs.iproute2
      pkgs.jq
      pkgs.systemd
      pkgs.util-linux
    ];
    text = builtins.readFile ../config/ai-dev-maintenance.sh;
  };
in {
  home.packages = [maintenance];

  xdg.configFile."fish/conf.d/coding-agent-aliases.fish" = {
    force = true;
    text = ''
      function codex --wraps=codex --description "alias codex codex --dangerously-bypass-approvals-and-sandbox"
          command codex --dangerously-bypass-approvals-and-sandbox $argv
      end

      function claude --wraps=claude --description "alias claude claude --dangerously-skip-permissions"
          command claude --dangerously-skip-permissions $argv
      end
    '';
  };

  systemd.user.services.moshi-hook = {
    Unit = {
      Description = "Moshi agent hooks and loopback gateway";
      Wants = ["network-online.target"];
      After = ["network-online.target"];
    };
    Service = {
      Type = "simple";
      ExecStart = "%h/.local/bin/moshi-hook serve";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = ["default.target"];
  };
}
