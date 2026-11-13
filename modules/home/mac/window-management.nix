{ config, lib, ... }:
{
  services = {
    # skhd finds ~/.config/skhd/skhdrc itself and watches the file the link
    # resolves to, so an edit in the checkout applies live.
    skhd.enable = true;

    # `borders` runs bordersrc only at launch; the module restarts the agent
    # when it changes. Try a value live with `borders <option>=<value>`.
    jankyborders = {
      enable = true;
      settings = {
        style = "round";
        width = 3.0;
        order = "above";
        hidpi = "on";
        # TokyoNight Night's cyan (#7dcfff).
        active_color = "0xff7dcfff";
        inactive_color = "0x00000000";
      };
    };
  };

  xdg.configFile."skhd/skhdrc".source = config.lib.nixConfig.file "config/skhd/skhdrc";

  # skhd runs each command through the login shell. Give it the PATH a shell
  # has, so `yabai` from nix-darwin resolves.
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
