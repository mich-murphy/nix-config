{
  lib,
  config,
  pkgs,
  inputs,
  ...
}: let
  cfg = config.common.minecraft;
  mcVersion = "1.21.1";
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
        servers.j-world = {
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
                url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/gQS3JbZO/fabric-api-0.103.0%2B1.21.1.jar";
                sha512 = "085e985d3000afb0d0d799fdf83f7f084dd240e9852ccb4d94ad13fc3d3fad90b00b02dcc493e3c38a66ae4757389582eccf89238569bacae638b9ffd9885ebc";
              };
              Lithium = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/gvQqBUqZ/versions/5szYtenV/lithium-fabric-mc1.21.1-0.13.0.jar";
                sha512 = "d4bd9a9cc37daad8828aa4fa9ca20e4f89d10e30cf6daf4546ef4cf4a684ba21ea0865a9c23cef9d1f4348e9ba4aca9aaca3db9f99534fc610fa78a5ca0bf151";
              };
              FerriteCore = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/uXXizFIs/versions/wmIZ4wP4/ferritecore-7.0.0-fabric.jar";
                sha512 = "0f2f9b5aebd71ef3064fc94df964296ac6ee8ea12221098b9df037bdcaaca7bccd473c981795f4d57ff3d49da3ef81f13a42566880b9f11dc64645e9c8ad5d4f";
              };
              Terralith = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/8oi3bsk5/versions/Mm6TmSwo/Terralith_1.21_v2.5.4.jar";
                sha512 = "d49f8a854a6ec8f49323986bbbffc061d62f2a85b91bc9ed442158c00551a66dcf8883b8151cfd732de8d6ba1006d0d94e6e456f15510eb32aacfd38da1095e1";
              };
            });
          };
        };
      };
    };
  };
}
