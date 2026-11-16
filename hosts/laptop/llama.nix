{
  config,
  lib,
  pkgs,
  ...
}:
let
  log = "${config.home.homeDirectory}/Library/Logs/llama-server.log";
in
{
  home.packages = [ pkgs.llama-cpp ];

  xdg.configFile."llama/models.ini".source = config.lib.nixConfig.file "config/llama/models.ini";

  home.sessionVariables.LLAMA_BASE_URL = "http://127.0.0.1:9931";

  # The router starts each model from the store path of its own binary. An
  # upgrade changes that path and so this plist, and Home Manager reloads an
  # agent whose plist changed, so the router restarts only on an upgrade.
  launchd.agents.llama-server = {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe' pkgs.llama-cpp "llama-server")
        "--models-preset"
        "${config.xdg.configHome}/llama/models.ini"
        "--models-max"
        "1"
      ];
      WorkingDirectory = "${config.home.homeDirectory}/models";
      RunAtLoad = true;
      KeepAlive = true;
      ProcessType = "Interactive";
      StandardOutPath = log;
      StandardErrorPath = log;
    };
  };
}
