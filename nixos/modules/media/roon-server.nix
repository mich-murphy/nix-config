{
  lib,
  pkgs,
  config,
  ...
}: let
  cfg = config.common.roon-server;
  version = "2.0-1470";
  urlVersion = builtins.replaceStrings ["." "-"] ["00" "0"] version;
in {
  imports = [
    ../borgbackup.nix
  ];

  options.common.roon-server = {
    enable = lib.mkEnableOption "Enable roon";
    extraGroups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Additional groups for roon user";
      example = ["media"];
    };
  };

  config = lib.mkIf cfg.enable {
    common.borgbackup.backupPaths = lib.mkIf config.common.borgbackup.enable ["/var/lib/roon-server"];

    services.roon-server = {
      enable = true;
      package = pkgs.roon-server.overrideAttrs (finalAttrs: previousAttrs: {
        src = builtins.fetchurl {
          url = "https://download.roonlabs.com/updates/production/RoonServer_linuxx64_${urlVersion}.tar.bz2";
          sha256 = "1vqlxqpqx6riviv9j5m11r9c0gsf7hmkjbgs6na5m5vg4ynv3iks";
        };
      });
    };

    users.users.roon-server.extraGroups = cfg.extraGroups;
  };
}
