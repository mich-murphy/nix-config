{ pkgs, ... }:
{
  imports = [ ./namespace.nix ];

  home = {
    username = "dev";
    homeDirectory = "/home/dev";
    stateVersion = "26.05";
    packages = [ pkgs.bubblewrap ];
    sessionVariables.TMPDIR = "/home/dev/businesscraft/.scratch";
  };

  systemd.user.systemctlPath = "/usr/bin/systemctl";

  nixConfig.checkout = null;

  programs.git.includes = [ { path = "~/.gitconfig-work"; } ];

  programs.home-manager.enable = true;
}
