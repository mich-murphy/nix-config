{
  lib,
  config,
  pkgs,
  inputs,
  ...
}: let
  cfg = config.common.minecraft;
  mcVersion = "1.20.1";
  serverVersion = lib.replaceStrings ["."] ["_"] "fabric-${mcVersion}";
in {
  imports = [
    inputs.nix-minecraft.nixosModules.minecraft-servers
  ];

  options.common.minecraft = {
    enable = lib.mkEnableOption "Enable Minecraft Server";
  };

  config = lib.mkIf cfg.enable {
    services = {
      minecraft-servers = {
        enable = true;
        eula = true;
        servers.pokemon = {
          enable = true;
          package = pkgs.fabricServers.${serverVersion};
          serverProperties = {
            server-port = 25565;
            gamemode = "creative";
            difficulty = "peaceful";
            pvp = false;
            motd = "J's World";
            max-players = 2;
            level-seed = "882427838104948496";
            spawn-monsters = false;
            enable-command-block = true;
          };
          symlinks = {
            # fetch mods from modrinth
            # hash identified: nix run github:Infinidoge/nix-minecraft#nix-modrinth-prefetch -- <version-id>
            mods = pkgs.linkFarmFromDrvs "mods" (builtins.attrValues {
              FabricApi = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/YG53rBmj/fabric-api-0.92.0%2B1.20.1.jar";
                sha512 = "53ce4cb2bb5579cef37154c928837731f3ae0a3821dd2fb4c4401d22d411f8605855e8854a03e65ea4f949dfa0e500ac1661a2e69219883770c6099b0b28e4fa";
              };
              Terralith = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/8oi3bsk5/versions/FgvUosFH/Terralith_1.20.4_v2.4.11.jar";
                sha512 = "b6bd6e71666dd57e1db19fe7cad85a9e0d9a13a9c6fa839c4e9cb149b12cf855f8f1676c4bc4d9647f1ba32a7a4f40d3f7090de3c5ddd569950c408b8cfd9128";
              };
            });
          };
        };
      };
    };
  };
}
