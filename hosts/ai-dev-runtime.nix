{...}: {
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

  # Weekly store garbage collection. Determinate Nix owns the binary, so call it
  # by its profile path rather than pulling a second nix into the closure.
  systemd.user.services.nix-gc = {
    Unit.Description = "Nix garbage collection";
    Service = {
      Type = "oneshot";
      ExecStart = "/nix/var/nix/profiles/default/bin/nix-collect-garbage --delete-older-than 30d";
    };
  };

  systemd.user.timers.nix-gc = {
    Unit.Description = "Weekly Nix garbage collection";
    Timer = {
      OnCalendar = "weekly";
      Persistent = true;
      RandomizedDelaySec = "1h";
    };
    Install.WantedBy = ["timers.target"];
  };
}
