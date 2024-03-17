{
  lib,
  config,
  pkgs,
  inputs,
  ...
}:
with lib; let
  cfg = config.common.minecraft;
  mcVersion = "1.20.1";
  # fabricVersion = "0.15.7";
  serverVersion = lib.replaceStrings ["."] ["_"] "fabric-${mcVersion}";
in {
  imports = [
    inputs.nix-minecraft.nixosModules.minecraft-servers
  ];

  options.common.minecraft = {
    enable = mkEnableOption "Enable Minecraft Server";
  };

  config = mkIf cfg.enable {
    services = {
      minecraft-servers = {
        enable = true;
        eula = true;
        servers.pokemon = {
          enable = true;
          package = pkgs.fabricServers.${serverVersion};
          serverProperties = {
            server-port = 25565;
            gamemode = "survival";
            difficulty = "peaceful";
            pvp = false;
            motd = "Pokemon Minecraft";
            max-players = 2;
            level-seed = "882427838104948496";
            spawn-monsters = false;
            enable-command-block = true;
          };
          symlinks = {
            mods = pkgs.linkFarmFromDrvs "mods" (builtins.attrValues {
              FabricApi = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/YG53rBmj/fabric-api-0.92.0%2B1.20.1.jar";
                sha512 = "53ce4cb2bb5579cef37154c928837731f3ae0a3821dd2fb4c4401d22d411f8605855e8854a03e65ea4f949dfa0e500ac1661a2e69219883770c6099b0b28e4fa";
              };
              Cobblemon = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/MdwFAVRL/versions/uWAkNUxZ/Cobblemon-fabric-1.4.1%2B1.20.1.jar";
                sha512 = "6955c8ad187d727cbfc51761312258600c5480878983cfe710623070c90eb437e419c140ff3c77e5066164876ecfe1e31b87f58f5ef175f0758efcff246b85a8";
              };
              Terralith = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/8oi3bsk5/versions/FgvUosFH/Terralith_1.20.4_v2.4.11.jar";
                sha512 = "b6bd6e71666dd57e1db19fe7cad85a9e0d9a13a9c6fa839c4e9cb149b12cf855f8f1676c4bc4d9647f1ba32a7a4f40d3f7090de3c5ddd569950c408b8cfd9128";
              };
              Waystones = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/LOpKHB2A/versions/36jYwMdH/waystones-fabric-1.20-14.1.3.jar";
                sha512 = "e89524a4ff449b4fcc8c4ee1538e060545d813160de77a700e7bd9ff4f1fb6cdda83828d29a8efdeb34bf12ce628d5ccc56c8e69e5b2ee5daff91462dfd1d929";
              };
            });
          };
        };
      };
    };
  };
}
