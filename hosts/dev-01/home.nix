{ pkgs, ... }:
{
  home = {
    username = "dev";
    homeDirectory = "/home/dev";
    # Set once at first activation; never bump it to track releases.
    stateVersion = "26.05";
    # Codex's sandbox on Linux.
    packages = [ pkgs.bubblewrap ];
    sessionVariables.TMPDIR = "/home/dev/businesscraft/.scratch";
  };

  # Nixpkgs' systemctl cannot reach Ubuntu's older user manager, so activation
  # would report it as not running and skip restarting changed services.
  systemd.user.systemctlPath = "/usr/bin/systemctl";

  nixConfig.checkout = null;

  programs.git.includes = [ { path = "~/.gitconfig-work"; } ];

  programs.home-manager.enable = true;
}
