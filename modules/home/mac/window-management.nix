{ config, lib, ... }:
let
  tokyonightNightCyan = "0xff7dcfff";
in
{
  services = {
    skhd.enable = true;

    jankyborders = {
      enable = true;
      settings = {
        style = "round";
        width = 3.0;
        order = "above";
        hidpi = "on";
        active_color = tokyonightNightCyan;
        inactive_color = "0x00000000";
      };
    };
  };

  xdg.configFile."skhd/skhdrc".source = config.lib.nixConfig.file "config/skhd/skhdrc";

  launchd.agents.skhd.config.EnvironmentVariables.PATH = lib.concatStringsSep ":" [
    "${config.home.profileDirectory}/bin"
    "/run/current-system/sw/bin"
    "/nix/var/nix/profiles/default/bin"
    "/usr/local/bin"
    "/usr/bin"
    "/bin"
    "/usr/sbin"
    "/sbin"
  ];
}
