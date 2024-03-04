{
  lib,
  config,
  pkgs,
  ...
}:
with lib; let
  cfg = config.common.minecraft;
in {
  options.common.minecraft = {
    enable = mkEnableOption "Enable Minecraft Server";
  };

  config = mkIf cfg.enable {
    services = {
      minecraft-server = {
        enable = true;
        eula = true;
        declarative = true;
        serverProperties = {
          server-port = 25565;
          gamemode = "survival";
          motd = "NixOS Minecraft Server";
          max-players = 2;
          level-seed = "10292758";
        };
      };
    };
  };
}
